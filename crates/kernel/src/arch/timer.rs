#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::timer::RiscvTimer as TimerImpl;

pub trait Timer {
    fn now() -> u64;
    fn set_deadline(t: u64);
}

#[inline(always)]
pub fn now() -> u64 {
    TimerImpl::now()
}

#[inline(always)]
pub fn set_deadline(t: u64) {
    TimerImpl::set_deadline(t);
}
