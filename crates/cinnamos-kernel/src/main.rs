#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use cinnamos_kernel::*;
use fdt::Fdt;

#[unsafe(no_mangle)]
unsafe extern "C" fn kernel_relocate(
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    unsafe {
        rel::relocate(dyn_ptr);
        entry(hid, dtb_ptr, dyn_ptr)
    }
}

unsafe fn entry(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("Failed to get interrupt ID for UART");
        device::uart::init(
            unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) },
            irq_id as u16,
        );
    }

    unsafe {
        hloc::load_boot_hart_local(hid);
        arch::init();
        mem::bump::init();
        mem::heap::init_bump();
        mem::palloc::init_bump();
        mem::vms::init().expect("Failed to initialize VMS");
        mem::vms::init_kernel_map(&fdt).expect("Failed to initialize virtual map");
        mem::vms::jump_higher_half(higher_half_entry as *const (), hid, dtb_ptr, dyn_ptr);
    }
}

unsafe extern "C" fn higher_half_entry(
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    unsafe {
        rel::shift_relocation(dyn_ptr, mem::vms::PHYS_TO_KERNEL_SLIDE);
    }
    mem::heap::shift_bump(&mem::vms::phys_to_virt);

    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("Invalid DTB") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("Failed to get interrupt ID for UART");
        let pa = arch::PAddr::from_ptr(uart_reg.start_ptr());
        device::uart::init(
            unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) },
            irq_id as u16,
        );
    }

    #[cfg(debug_assertions)]
    println!("debug : higher-half entry (HID {})", hid);
    unsafe {
        hloc::load_boot_hart_local(hid);
        arch::init_higher_half();
        mem::palloc::init(&fdt, dtb_ptr);
        mem::vms::uninit_identity_map().expect("Failed to uninitialize identity map");
        mem::heap::init_heap().expect("Failed to initialize heap allocator");
    }

    arch::init_interrupts(hid, &fdt);

    #[cfg(debug_assertions)]
    println!("debug : waiting for interrupt (HID {})", hid);
    loop {
        arch::wait_for_interrupt();
    }
}
