#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    cinnamos_kernel::arch::init();

    cinnamos_kernel::arch::console::clear();
    cinnamos_kernel::arch::console::putstr("Hello World!\n");

    cinnamos_kernel::cpu::idle();
}
