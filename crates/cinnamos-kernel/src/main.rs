#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use cinnamos_kernel::{
    arch::{PAddr, VAddr},
    mem::{PhysFrameAlloc, SizedMemoryRegion},
    sym::*,
    *,
};
use fdt::Fdt;

/// Fills in the global offset table for dynamic symbol relocations and calls the kernel entry function.
///
/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol.
#[unsafe(no_mangle)]
unsafe extern "C" fn kernel_relocate(
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    // Safety: dyn_ptr points to _DYNAMIC
    unsafe { rel::relocate(dyn_ptr) };
    // Safety: All arguments forwarded from parameters
    unsafe { entry(hid, dtb_ptr, dyn_ptr) };
}

/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol.
unsafe fn entry(hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    let trap_stack = mem::physalloc::alloc(2).expect("failed to allocate trap stack");
    let tsp = VAddr::identity(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init();

    // Safety: dtb_ptr points to a devicetree blob
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("failed to get interrupt ID for UART");
        io::serial::init(
            // Safety: uart_reg does not have a null base address
            unsafe { NonNull::new_unchecked(uart_reg.start_ptr().cast_mut()) },
            irq_id as u16,
        );
    }
    klog::init();

    mem::vms::init(&fdt, PAddr::from_ptr(dtb_ptr)).expect("failed to initialize VMS");
    klog::disable();
    unsafe { relocate_jump_higher_half(entry_virt as *const (), hid, dtb_ptr, dyn_ptr) };
}

/// Shifts all relocations to virtual space and jumps to `entry`.
///
/// # Safety
/// - The virtual kernel and direct map space must be mapped.
/// - `entry` must point to the physical address of a function, which is mapped in kernel space.
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob, which is mapped in direct map space.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol, which is mapped in kernel space.
unsafe fn relocate_jump_higher_half(
    entry: *const (),
    hid: usize,
    dtb_ptr: *const u8,
    dyn_ptr: *const rel::Elf64Dyn,
) -> ! {
    let ventry = mem::vms::phys_to_kernel(PAddr::from_ptr(entry));
    let vdtb = mem::vms::phys_to_virt(PAddr::from_ptr(dtb_ptr));
    let vdyn = mem::vms::phys_to_kernel(PAddr::from_ptr(dyn_ptr));
    let vsp = mem::vms::phys_to_kernel(stack_end_p());

    // Safety: vdyn is the virtual address of _DYNAMIC
    unsafe { rel::shift_relocation(vdyn.as_ptr(), mem::vms::PHYS_TO_KERNEL_SLIDE) };
    // Safety: Safety conditions are fulfilled in parameters
    unsafe { arch::jump_higher_half(ventry.as_ptr(), hid, vdtb, vsp) };
}

/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the virtual location of a devicetree blob.
unsafe fn entry_virt(hid: usize, dtb_ptr: *const u8) -> ! {
    // Safety: Bump space is mapped into kernel space
    unsafe { mem::heap::shift_bump(&mem::vms::phys_to_kernel) };

    // Safety: dtb_ptr points to a devicetree blob
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("failed to get interrupt ID for UART");
        let pa = PAddr::from_ptr(uart_reg.start_ptr());
        io::serial::init(
            unsafe { NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()) },
            irq_id as u16,
        );
    }
    klog::enable();
    log::info!("higher-half entry");

    let trap_stack = mem::physalloc::alloc(4).expect("failed to allocate trap stack");
    let tsp = mem::vms::phys_to_kernel(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init_higher_half();

    mem::physalloc::init(&fdt, mem::vms::virt_to_phys(VAddr::from_ptr(dtb_ptr)));
    mem::vms::remap_tables().expect("failed to remap to higher-half");
    mem::heap::init_heap();
    mem::vms::uninit_identity_map().expect("failed to uninitialize identity map");

    let (bump_start, bump_next, bump_end) = mem::alloc::bump::get_bump_area();
    log::info!(
        "bump area={:#016x} .. {:#016x}, head={:#016x}, used={}/{}",
        bump_start,
        bump_end,
        bump_next,
        bump_next - bump_start,
        bump_end - bump_start,
    );
    if let Some(reg) = SizedMemoryRegion::from_range(
        bump_next.align_to_next_page(),
        bump_end.align_to_next_page(),
    ) {
        mem::physalloc::add_region(&reg);
    }

    // Safety: idle is a callable function
    let task = unsafe { task::new_kernel_task(idle as _).expect("failed to create idle task") };
    // Safety: task already has a context
    unsafe { sched::enqueue(task) };
    arch::init_interrupts(hid, &fdt);

    log::info!("starting scheduler");
    sched::start();
}

/// An idle task that simply yields. Required to make sure the scheduler run queue is never empty.
fn idle() -> ! {
    log::trace!("hello from idle()");
    loop {
        if let Err(e) = sys::thread::thread_yield() {
            log::warn!("idle(): thread_yield returned with error {:?}", e);
        }
        log::trace!("hello from idle()::loop");
        arch::wait_for_interrupt();
    }
}
