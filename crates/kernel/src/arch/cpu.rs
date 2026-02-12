#[cfg(target_arch = "riscv32")]
pub use crate::arch::riscv::cpu::RiscvCpu as CpuImpl;
#[cfg(target_arch = "riscv32")]
pub use crate::arch::riscv::cpu::RiscvCpuLocal as CpuLocalImpl;

use crate::cpu::local::CpuLocal;

pub trait Cpu<C : CpuLocal> {
    fn init(id: usize);

    fn local() -> &'static C;
    fn local_mut() -> &'static mut C;

    fn idle() -> !;
}

#[inline(always)]
pub fn init(id: usize) {
    CpuImpl::init(id);
}

#[inline(always)]
pub fn local() -> &'static CpuLocalImpl {
    CpuImpl::local()
}

#[inline(always)]
pub fn local_mut() -> &'static mut CpuLocalImpl {
    CpuImpl::local_mut()
}

#[inline(always)]
pub fn idle() -> ! {
    CpuImpl::idle();
}
