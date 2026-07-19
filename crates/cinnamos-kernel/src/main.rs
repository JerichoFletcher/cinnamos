#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use cinnamos_kernel::{
    arch::{PAddr, VAddr},
    *,
};
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
    hloc::load_boot_hart_local(hid);
    arch::init();

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

    mem::bump::init();
    mem::heap::init_bump();
    mem::palloc::init_bump();
    mem::vms::init().expect("Failed to initialize VMS");
    mem::vms::init_kernel_map(&fdt, PAddr::from_ptr(dtb_ptr))
        .expect("Failed to initialize virtual map");
    unsafe {
        jump_higher_half(higher_half_entry as *const (), hid, dtb_ptr, dyn_ptr);
    }
}

/// # Safety
/// - `entry` must point to a physical location and be virtually mapped.
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to a physical location and be direct-mapped.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol and be direct-mapped.
unsafe fn jump_higher_half(
    entry: *const (),
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    let ventry = mem::vms::phys_to_kernel(PAddr::from_ptr(entry));
    let vdtb = mem::vms::phys_to_virt(PAddr::from_ptr(dtb_ptr));
    let vdyn = mem::vms::phys_to_kernel(PAddr::from_ptr(dyn_ptr));
    let vsp = mem::vms::phys_to_kernel(unsafe { stack_end_p!() });
    unsafe {
        arch::jump_higher_half(ventry.as_ptr(), hid, vdtb, vdyn, vsp);
    }
}

unsafe fn higher_half_entry(
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    unsafe {
        rel::shift_relocation(dyn_ptr, mem::vms::PHYS_TO_KERNEL_SLIDE);
    }
    hloc::load_boot_hart_local(hid);
    arch::init_higher_half();
    mem::heap::shift_bump(&mem::vms::phys_to_kernel);

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

    mem::palloc::init(&fdt, mem::vms::virt_to_phys(VAddr::from_ptr(dtb_ptr)));
    mem::vms::uninit_identity_map().expect("Failed to uninitialize identity map");

    if let Some((bump_start, bump_next, bump_end)) = mem::bump::get_bump_area() {
        println!(
            "bump : area=0x{:016x} .. 0x{:016x}, head=0x{:016x}, used={}/{}",
            bump_start,
            bump_end,
            bump_next,
            bump_next - bump_start,
            bump_end - bump_start,
        );
    }
    mem::heap::init_heap().expect("Failed to initialize heap allocator");

    arch::init_interrupts(hid, &fdt);

    #[cfg(debug_assertions)]
    println!("debug : waiting for interrupt (HID {})", hid);
    loop {
        arch::wait_for_interrupt();
    }
}
