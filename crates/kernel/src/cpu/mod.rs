pub mod interrupt;

use crate::arch::cpu;

pub fn idle() -> ! {
    cpu::idle();
}
