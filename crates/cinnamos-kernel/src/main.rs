#![no_std]
#![no_main]

use cinnamos_kernel::*;
use fdt::Fdt;

#[unsafe(no_mangle)]
extern "C" fn main(_hid: usize, dtb_ptr: *const u8) -> ! {
    let dtb = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&dtb, &["ns16550", "ns16550a"]) {
        device::uart::init(uart_reg.base);
    }

    for n in dtb.all_nodes() {
        if let Some(compat) = n.compatible() {
            print!("[NODE] {} ->", n.name);
            for c in compat.all() {
                print!(" {}", c);
            }
            println!();
        }
    }

    arch::init();
    loop {}
}
