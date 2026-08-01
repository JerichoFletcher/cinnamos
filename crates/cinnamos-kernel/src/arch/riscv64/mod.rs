use core::{arch::asm, ptr::NonNull};

use fdt::Fdt;

use crate::{devicetree, mem, *};

pub mod context;
pub mod device;
pub mod hloc;
pub mod interrupt;
pub mod paddr;
pub mod sv48;
pub mod timer;
pub mod trap;
pub mod vaddr;

use paddr::PAddr;
use vaddr::VAddr;

#[inline]
pub fn wait_for_interrupt() {
    riscv::asm::wfi();
}

pub fn init() {
    trap::init();
}

pub fn init_higher_half() {
    trap::init_higher_half();
}

pub fn init_interrupts(hid: usize, fdt: &Fdt) {
    if let Some(plic_node) = fdt.find_compatible(&["riscv,plic0"])
        && let Some(mut plic_reg) = plic_node.reg()
    {
        let plic_reg = plic_reg.next().unwrap();
        let pa = PAddr::from_ptr(plic_reg.starting_address);

        device::plic::init(unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) });
        let mut plic = device::plic::get_plic_mut();
        for (node, ints) in devicetree::all_with_interrupts(fdt, &plic_node) {
            for int in ints {
                println!("plic : enabling interrupt {}: {}", int, node.name);
                plic.set_priority(int as u16, 1);
                plic.set_enabled(int as u16, hid, true);
            }
        }
        plic.set_threshold(hid, 0);
    }

    timer::schedule_timer();
    interrupt::enable_interrupts();
}

/// # Safety
/// - `target`, `dtb_ptr`, `dyn_ptr`, and `new_sp` must be within the initialized higher-half virtual map.
/// - `hid` must be equal to the executing hart ID.
pub unsafe fn jump_higher_half(
    target: *const (),
    hid: usize,
    dtb_ptr: VAddr,
    dyn_ptr: VAddr,
    new_sp: VAddr,
) -> ! {
    unsafe {
        asm!(
            "mv sp, {sp}",
            "jr t0",
            sp = in(reg) new_sp.addr(),
            in("a0") hid,
            in("a1") dtb_ptr.addr(),
            in("a2") dyn_ptr.addr(),
            in("t0") target,
            options(noreturn),
        );
    }
}
