use crate::arch::aimpl;

#[inline]
pub fn wait_for_interrupt() {
    aimpl::cpu::wait_for_interrupt();
}
