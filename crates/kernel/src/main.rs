#![no_std]
#![no_main]

use cinnamos_kernel::*;

#[unsafe(no_mangle)]
pub extern "C" fn kmain(id: usize, _dtb_ptr: usize) -> ! {
    cpu::init(id);

    arch::init();
    page::init();
    mem::init();

    time::init();
    sched::init();
    
    cpu::idle();
}
