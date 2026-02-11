#![no_std]
#![no_main]

use cinnamos_kernel::*;

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    arch::init();
    cpu::idle();
}
