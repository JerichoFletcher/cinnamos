#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use fdt::Fdt;
use cinnamos_kernel::*;

#[unsafe(no_mangle)]
unsafe extern "C" fn kernel_relocate(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    unsafe {
        rel::relocate(dyn_ptr);
        entry(hid, dtb_ptr, dyn_ptr)
    }
}

unsafe fn entry(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&fdt, &["ns16550", "ns16550a"]) {
        device::uart::init(unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) });
    }
    
    arch::init();
    mem::palloc::init(&fdt, dtb_ptr);
    mem::vms::init().unwrap();
    match mem::vms::init_kernel_map(&fdt) {
        Ok(_) => unsafe {
            mem::vms::jump_higher_half(higher_half_entry as *const (), hid, dtb_ptr, dyn_ptr);
        },
        Err(e) => panic!("{:?}", e),
    }
}

unsafe extern "C" fn higher_half_entry(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    unsafe { rel::shift(dyn_ptr, mem::vms::PHYS_TO_KERNEL_SLIDE); }

    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some(uart_reg) = dtb::find_compatible_region(&fdt, &["ns16550", "ns16550a"]) {
        let pa = arch::PAddr::from_ptr(uart_reg.start_ptr());
        device::uart::init(unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) });
    }

    #[cfg(debug_assertions)] {
        println!("debug : higher-half entry (HID {})", hid);
    }

    arch::init_higher_half();
    mem::palloc::reinit_higher_half().unwrap();
    unsafe { mem::vms::uninit_identity_map().unwrap() };
    mem::heap::init().unwrap();

    loop { arch::wait_for_interrupt(); }
}
