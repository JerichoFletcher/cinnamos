#[cfg(target_arch = "riscv32")]
pub mod riscv;

pub mod cpu;
pub mod context;
pub mod trap;
pub mod time;
pub mod console;

#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::RiscvArch as ArchImpl;

pub trait Arch {
    fn init();
}

#[inline(always)]
pub fn init() {
    ArchImpl::init();
}
