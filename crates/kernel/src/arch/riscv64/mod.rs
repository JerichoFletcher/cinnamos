use core::{arch::asm, ptr::NonNull};

use alloc::boxed::Box;
use elf::dynamic::Elf64_Dyn;
use fdt::Fdt;

use crate::{
    arch::{
        ic::{InterruptController, InterruptPriority},
        interrupt::{InterruptPriorityThreshold, interrupt_free},
        timer::schedule_timer,
    },
    devicetree, mem,
};

mod asm;

pub mod console;
pub mod context;
pub mod hart;
pub mod hloc;
pub mod ic;
pub mod interrupt;
pub mod paddr;
pub mod sv48;
pub mod task;
pub mod timer;
pub mod trap;
pub mod vaddr;

use paddr::PAddr;
use vaddr::VAddr;

pub type ElfDyn = Elf64_Dyn;

/// Halts the current hart until an interrupt might need servicing. Does not guarantee that an interrupt
/// is serviceable, so callers must not assume such. However, the hart is guaranteed to continue execution
/// when an enabled interrupt is pending.
#[inline]
pub fn wait_for_interrupt() {
    riscv::asm::wfi();
}

/// Loads the address of `_DYNAMIC` using pure PC-relative address loading strategy. Because this load
/// skips the global offset table entirely, it is safe to use before any symbol relocation is performed.
#[inline(always)]
pub fn get_dyn() -> *const ElfDyn {
    let ptr: *const ElfDyn;
    unsafe {
        asm!(
            "lla {}, _DYNAMIC",
            out(reg) ptr,
            options(nomem, nostack),
        )
    };
    ptr
}

/// Performs any necessary architecture-specific initializations.
pub fn init() {
    trap::init();
}

/// Performs any necessary architecture-specific initializations, strictly after higher-half mapping
/// is enabled on the current hart.
pub fn init_higher_half() {
    trap::init();
}

/// # Safety
/// - `target`, `dtb_ptr`, `dyn_ptr`, and `new_sp` must be within the initialized higher-half virtual map.
/// - `hid` must be equal to the executing hart ID.
pub unsafe fn jump_higher_half(target: *const (), hid: usize, dtb_ptr: VAddr, new_sp: VAddr) -> ! {
    unsafe {
        asm!(
            "mv sp, {sp}",
            "jr t0",
            sp = in(reg) new_sp.addr(),
            in("a0") hid,
            in("a1") dtb_ptr.addr(),
            in("t0") target,
            options(noreturn),
        );
    }
}

/// Initializes interrupt controllers.
///
/// # Safety
/// `hid` must be equal to the current hart ID.
pub unsafe fn init_interrupt_driver(hid: usize, fdt: &Fdt) {
    if let Some(plic_node) = fdt.find_compatible(&["riscv,plic0"])
        && let Some(mut plic_reg) = plic_node.reg()
    {
        let plic_reg = plic_reg.next().unwrap();
        let pa = PAddr::from_ptr(plic_reg.starting_address);

        let plic_ptr = NonNull::new(mem::vms::phys_to_virt(pa).as_mut()).expect("plic is null");
        // Safety: plic_ptr is direct-mapped to PLIC region
        let mut plic = unsafe { ic::plic::Plic::new(plic_ptr) };
        interrupt_free(|ms| {
            for (node, ints) in devicetree::all_with_interrupts(fdt, &plic_node) {
                for int in ints {
                    log::debug!("enabling interrupt {}: {}", int, node.name);
                    plic.set_priority(ms, int, InterruptPriority::Low);
                }
            }
            ic::set_controller(ms, Box::new(plic));
        });
    }
    init_timer(hid, fdt);
}

/// Initializes and enables interrupts for this hart.
pub fn init_interrupts() {
    interrupt_free(|ms| {
        if let Some(ic) = ic::get_controller_read(ms).as_ref() {
            ic.set_threshold(ms, InterruptPriorityThreshold::Low);
            for descriptor in ic.iter_enabled_interrupts(ms) {
                ic.set_enabled(ms, descriptor.source, true);
            }
        }

        schedule_timer();
        interrupt::enable_interrupts();
    });
}

/// Initializes timer interrupts. Depends on the `timebase-frequency` property on the current hart
/// to determine the cycle interval between interrupts.
pub fn init_timer(hid: usize, fdt: &Fdt) {
    let cpu = fdt
        .cpus()
        .find(|cpu| cpu.ids().all().any(|id| id == hid))
        .expect("missing devicetree /cpus entry");
    timer::init_timer(cpu.timebase_frequency() / 100);
}
