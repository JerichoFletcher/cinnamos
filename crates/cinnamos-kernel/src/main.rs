#![no_std]
#![no_main]

use core::ptr::NonNull;

use cinnamos_kernel::*;
use fdt::Fdt;

fn print_dt(fdt: &Fdt) {
    for n in fdt.all_nodes() {
        print!("node : {}", n.name);

        if let Some(ints) = n.interrupts() {
            print!("; int=");
            let mut first = true;
            for i in ints {
                if first { print!("{}", i) } else { print!(",{}", i) }
                first = false;
            }
        }

        if let Some(int_prnt) = n.interrupt_parent() {
            print!("; intp={}", int_prnt.name);
        }

        if let Some(compat) = n.compatible() {
            print!("; compat=");
            for c in compat.all() {
                print!("[{}]", c);
            }
        }
        println!();
    }
}

#[unsafe(no_mangle)]
extern "C" fn main(_hid: usize, dtb_ptr: *const u8) -> ! {
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&fdt, &["ns16550", "ns16550a"]) {
        device::uart::init(unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) });
    }
    print_dt(&fdt);

    arch::init();
    mem::palloc::init(&fdt);

    loop {}
}
