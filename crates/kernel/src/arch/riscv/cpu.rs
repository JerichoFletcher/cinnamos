use riscv::asm::wfi;

pub use crate::arch::riscv::trap::set_interrupt_mask;

#[inline(always)]
pub fn idle() -> ! {
    loop {
        wfi();
    }
}
