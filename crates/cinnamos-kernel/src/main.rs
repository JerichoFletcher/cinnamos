#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use cinnamos_kernel::{
    arch::{PAddr, VAddr},
    sym::*,
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
        entry(hid, dtb_ptr, dyn_ptr);
    }
}

/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol.
unsafe fn entry(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    hloc::load_boot_hart_local(hid, VAddr::identity(trap_stack_end_p()));
    arch::init();

    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("failed to get interrupt ID for UART");
        device::uart::init(
            unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) },
            irq_id as u16,
        );
    }
    klog::init();

    mem::vms::init(&fdt, PAddr::from_ptr(dtb_ptr)).expect("failed to initialize VMS");
    unsafe {
        jump_higher_half(higher_half_entry as *const (), hid, dtb_ptr, dyn_ptr);
    }
}

/// # Safety
/// - `entry` must point to the virtual address of a function.
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the virtual location of a devicetree blob.
/// - `dyn_ptr` must point to the virtual `_DYNAMIC` symbol.
unsafe fn jump_higher_half(
    entry: *const (),
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    let ventry = mem::vms::phys_to_kernel(PAddr::from_ptr(entry));
    let vdtb = mem::vms::phys_to_virt(PAddr::from_ptr(dtb_ptr));
    let vdyn = mem::vms::phys_to_kernel(PAddr::from_ptr(dyn_ptr));
    let vsp = mem::vms::phys_to_kernel(stack_end_p());

    unsafe {
        rel::shift_relocation(vdyn.as_ptr(), mem::vms::PHYS_TO_KERNEL_SLIDE);
    }
    unsafe {
        arch::jump_higher_half(ventry.as_ptr(), hid, vdtb, vsp);
    }
}

unsafe fn higher_half_entry(hid: usize, dtb_ptr: *const u8) -> ! {
    hloc::load_boot_hart_local(hid, trap_stack_end_v());
    arch::init_higher_half();
    mem::heap::shift_bump(&mem::vms::phys_to_kernel);

    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("failed to get interrupt ID for UART");
        let pa = PAddr::from_ptr(uart_reg.start_ptr());
        device::uart::init(
            unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) },
            irq_id as u16,
        );
    }
    log::info!("higher-half entry");

    mem::physalloc::init(&fdt, mem::vms::virt_to_phys(VAddr::from_ptr(dtb_ptr)));
    mem::vms::remap_tables().expect("failed to remap to higher-half");
    mem::heap::init_heap();
    mem::vms::uninit_identity_map().expect("failed to uninitialize identity map");

    if let Some((bump_start, bump_next, bump_end)) = mem::alloc::bump::get_bump_area() {
        log::info!(
            "bump area=0x{:016x} .. 0x{:016x}, head=0x{:016x}, used={}/{}",
            bump_start,
            bump_end,
            bump_next,
            bump_next - bump_start,
            bump_end - bump_start,
        );
    }
    sched::enqueue(task::new_kernel_task(idle as _).expect("failed to create idle task"));
    arch::init_interrupts(hid, &fdt);

    log::info!("starting scheduler");
    sched::start();
}

fn idle() -> ! {
    log::debug!("hello from idle()");
    loop {
        if let Err(e) = sys::thread::thread_yield() {
            log::warn!("idle(): thread_yield returned with error {:?}", e);
        }
        log::trace!("hello from idle()::loop");
        arch::wait_for_interrupt();
    }
}
