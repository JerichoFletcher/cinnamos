use core::{arch::asm, ptr::NonNull};

use fdt::Fdt;
use riscv::register::{sepc, sstatus};

use crate::{devicetree, mem, *};

mod trap;

pub mod hloc;
pub mod device;
pub mod context;
pub mod timer;
pub mod paddr;
pub mod vaddr;
pub mod sv48;
pub mod interrupt;

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
    
    timer::schedule_timer();
    timer::enable_timer();
}

pub fn init_interrupts(hid: usize, fdt: &Fdt) {
    if let Some(plic_node) = fdt.find_compatible(&["riscv,plic0"]) && let Some(mut plic_reg) = plic_node.reg() {
        let plic_reg = plic_reg.next().unwrap();
        let pa = PAddr::from_ptr(plic_reg.starting_address);
    
        device::plic::init(unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) });
        device::plic::acquire(|plic| {
            for (node, ints) in devicetree::all_with_interrupts(fdt, &plic_node) {
                for int in ints {
                    println!("plic : enabling interrupt {}: {}", int, node.name);
                    plic.set_priority(int as u16, 1);
                    plic.set_enabled(int as u16, hid, true);
                }
            }
            plic.set_threshold(hid, 0);
        }).ok();
    }
}

/// # Safety
/// - `target`, `dtb_ptr`, `dyn_ptr`, and `new_sp` must be within the initialized higher-half virtual map.
/// - `hid` must be equal to the executing hart ID.
pub unsafe fn jump_higher_half(target: *const (), hid: usize, dtb_ptr: VAddr, dyn_ptr: VAddr, new_sp: VAddr) -> ! {
    unsafe {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::Supervisor);
        sstatus.set_spie(sstatus.sie());
        sstatus::write(sstatus);
        sepc::write(target as usize);

        asm!(
            "mv sp, {0}",
            "mv a0, {1}",
            "mv a1, {2}",
            "mv a2, {3}",
            in(reg) new_sp.addr(),
            in(reg) hid,
            in(reg) dtb_ptr.addr(),
            in(reg) dyn_ptr.addr(),
        );
        asm!("sret", options(noreturn));
    }
}
