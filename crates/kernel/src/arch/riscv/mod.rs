use crate::arch::Arch;

pub mod cpu;
pub mod context;
pub mod trap;
pub mod timer;
pub mod console;

mod sbi;

pub struct RiscvArch;

impl Arch for RiscvArch {
    fn init() {
        sbi::init();
        trap::init();
    }
}
