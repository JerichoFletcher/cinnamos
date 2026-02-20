use crate::arch::riscv32::trap;

pub fn init() {
    trap::init();
}
