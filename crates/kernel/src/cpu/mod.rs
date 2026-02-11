pub mod interrupt;

#[inline(always)]
pub fn idle() -> ! {
    crate::arch::cpu::idle();
}
