use crate::arch::Arch;
use crate::arch::riscv::{sbi, trap};

pub struct RiscvArch;

impl Arch for RiscvArch {
    fn init() {
        sbi::init();
        trap::init();
    }
}
