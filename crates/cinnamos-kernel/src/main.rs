#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use alloc::{string::String, vec};
use fdt::Fdt;
use cinnamos_kernel::*;

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
extern "C" fn main(hid: usize, dtb_ptr: *const u8) -> ! {
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&fdt, &["ns16550", "ns16550a"]) {
        device::uart::init(unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) });
    }
    print_dt(&fdt);

    arch::init();
    mem::palloc::init(&fdt, dtb_ptr);
    mem::vms::init().unwrap();
    match mem::vms::init_kernel_map(&fdt) {
        Ok(_) => unsafe { mem::vms::jump_higher_half(higher_half_entry as *const (), hid, dtb_ptr); },
        Err(e) => panic!("{:?}", e),
    }
}

extern "C" fn higher_half_entry(_hid: usize, dtb_ptr: *const u8) -> ! {
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&fdt, &["ns16550", "ns16550a"]) {
        let pa = arch::PAddr::from_ptr(uart_reg.start_ptr());
        device::uart::init(unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) });
    }

    arch::init_higher_half();
    mem::palloc::reinit_higher_half().unwrap();
    mem::heap::init().unwrap();

    {
        let s = String::from("Hello, World!");
        let v = vec![67, 67, 67, 67, 67, 67];
        println!("{:?}", s);
        println!("{:?}", v);
    }

    loop {}
}
