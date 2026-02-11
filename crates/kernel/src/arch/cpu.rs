#[cfg(target_arch = "riscv32")]
pub use crate::arch::riscv::cpu::RiscvCpu as CpuImpl;

pub trait Cpu {
    fn idle() -> !;
}

#[inline(always)]
pub fn idle() -> ! {
    CpuImpl::idle();
}
