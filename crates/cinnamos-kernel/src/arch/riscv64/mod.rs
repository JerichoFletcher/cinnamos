use core::arch::asm;

use riscv::register::{sepc, sstatus};

use crate::arch::VAddr;

mod trap;

pub mod context;
pub mod timer;
pub mod paddr;
pub mod vaddr;
pub mod sv48;

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
