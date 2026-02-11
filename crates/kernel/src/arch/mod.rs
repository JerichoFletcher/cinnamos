#[cfg(target_arch = "riscv32")]
pub mod riscv;

#[cfg(target_arch = "riscv32")]
pub use crate::arch::riscv::*;

pub mod cpu;
pub mod context;
pub mod trap;
pub mod console;
