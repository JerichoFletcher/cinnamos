#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::time::RiscvTime as TimerImpl;

pub trait Time {
    fn now() -> u64;
    fn has_timer() -> bool;
    fn deadline() -> u64;
    fn set_deadline(t: u64);
}

#[inline(always)]
pub fn now() -> u64 {
    TimerImpl::now()
}

#[inline(always)]
pub fn has_timer() -> bool {
    TimerImpl::has_timer()
}

#[inline(always)]
pub fn deadline() -> u64 {
    TimerImpl::deadline()
}

#[inline(always)]
pub fn set_deadline(t: u64) {
    TimerImpl::set_deadline(t);
}
