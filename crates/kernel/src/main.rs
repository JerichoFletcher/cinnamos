#![no_std]
#![no_main]

use cinnamos_kernel::*;

#[unsafe(no_mangle)]
pub extern "C" fn kmain(id: usize) -> ! {
    cpu::init(id);
    arch::init();
    time::init();
    
    cpu::idle();
}
