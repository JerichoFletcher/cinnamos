#[cfg(target_arch = "riscv32")]
mod riscv32;

#[cfg(target_arch = "riscv32")]
use crate::arch::riscv32 as aimpl;

mod arch;
pub use arch::*;

pub mod cpu;
pub mod mem;
pub mod boot;
pub mod context;
