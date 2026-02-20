#[cfg(target_arch = "riscv32")]
pub mod riscv;
#[cfg(target_arch = "riscv32")]
pub use crate::arch::riscv::RiscvArch as ArchImpl;

pub mod cpu;
pub mod mem;
pub mod context;
pub mod trap;
pub mod time;
pub mod console;

mod arch;
pub use arch::*;
