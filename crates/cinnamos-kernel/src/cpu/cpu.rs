#[inline]
pub fn wait_for_interrupt() {
    crate::arch::cpu::wait_for_interrupt();
}

#[inline]
pub fn idle() -> ! {
    loop { wait_for_interrupt(); }
}
