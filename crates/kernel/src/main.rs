#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use cinnamos_kernel::{
    arch::addr::{PAddr, VAddr},
    mem::{PhysFrameAlloc, SizedMemoryRegion},
    sym::*,
    *,
};
use fdt::Fdt;

unsafe extern "C" {
    /// Entry point of SMP harts.
    ///
    /// # Safety
    /// - `hid` must be equal to the ID of the executing hart.
    /// - `stack_pa` must point to the top of a valid stack.
    fn _kernel_smp_start(hid: usize, stack_pa: PAddr);
}

/// Fills in the global offset table for dynamic symbol relocations and calls the kernel entry function.
///
/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob.
#[unsafe(no_mangle)]
unsafe extern "C" fn kernel_relocate(hid: usize, dtb_ptr: *const u8) -> ! {
    rel::relocate();
    // Safety: All arguments forwarded from parameters
    unsafe { entry(hid, dtb_ptr) };
}

/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob.
unsafe fn entry(hid: usize, dtb_ptr: *const u8) -> ! {
    let trap_stack = mem::physalloc::alloc(2).expect("failed to allocate trap stack");
    let tsp = VAddr::identity(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init();
    klog::init();
    log::info!("boot hart pentry hid={}", hid);

    // Safety: dtb_ptr points to a devicetree blob
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    let hart_count = fdt.cpus().count();
    unsafe { hart::set_hart_count(hart_count) };

    mem::vms::init(fdt, PAddr::from_ptr(dtb_ptr)).expect("failed to initialize VMS");
    let (smp_entry_pa, _) =
        mem::vms::translate_virt(VAddr::from_ptr(_kernel_smp_start as *const ()))
            .expect("failed to get physical address for SMP entry");
    log::info!(
        "starting sibling harts on SMP entry at {:#016x}",
        smp_entry_pa
    );
    for id in 0..hart_count {
        if id != hid {
            let stack = mem::physalloc::alloc(8).expect("failed to allocate SMP stack");
            let r = unsafe { arch::start_hart(id, _kernel_smp_start as _, stack) };
            log::info!("start hid={} status={:?}", id, r);
        }
    }

    // Barrier block until all SMP harts are started
    if hart::wait_all_harts_finalize(|| {
        log::trace!("synchronized relocation");
        // Safety: PHYS_TO_KERNEL_SLIDE is the kernel space's slide amount
        unsafe { rel::shift_relocation(mem::vms::PHYS_TO_KERNEL_SLIDE) };
        // Safety: Bump space is mapped into kernel space
        unsafe { mem::heap::shift_bump(mem::vms::phys_to_kernel) };
    })
    .is_err()
    {
        panic!("multiple finalizers for relocation barrier");
    }

    unsafe { jump_higher_half(entry_virt as *const (), hid, dtb_ptr, stack_end_p()) };
}

/// Shifts all relocations to virtual space and jumps to `entry`.
///
/// # Safety
/// - The virtual kernel and direct map space must be mapped.
/// - `entry` must point to the physical address of a function, which is mapped in kernel space.
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the physical location of a devicetree blob, which is mapped in direct map space.
unsafe fn jump_higher_half(entry: *const (), hid: usize, dtb_ptr: *const u8, sp: PAddr) -> ! {
    let ventry = mem::vms::phys_to_kernel(PAddr::from_ptr(entry));
    let vdtb = mem::vms::phys_to_virt(PAddr::from_ptr(dtb_ptr));
    let vsp = mem::vms::phys_to_kernel(sp);

    // Safety: Safety conditions are fulfilled in parameters
    unsafe { arch::jump_higher_half(ventry.as_ptr(), hid, vdtb, vsp) };
}

/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to the virtual location of a devicetree blob.
unsafe fn entry_virt(hid: usize, dtb_ptr: *const u8) -> ! {
    log::info!("boot hart ventry hid={}", hid);

    // Safety: dtb_ptr points to a devicetree blob
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("invalid devicetree blob") };
    if let Some((uart, uart_reg)) = devicetree::find_compatible(&fdt, &["ns16550", "ns16550a"]) {
        let irq_id = uart
            .interrupts()
            .map(|mut c| c.next().unwrap_or(0))
            .expect("failed to get interrupt ID for UART");
        let pa = PAddr::from_ptr(uart_reg.start_ptr());

        // Safety: uart_reg is a non-null address of the serial IO UART region
        unsafe {
            io::serial::init(
                NonNull::new_unchecked(mem::vms::phys_to_virt(pa).as_mut()),
                arch::interrupt::InterruptSource::new(irq_id),
            )
        };
    }

    let trap_stack = mem::physalloc::alloc(32).expect("failed to allocate trap stack");
    let tsp = mem::vms::phys_to_kernel(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init_higher_half();

    // Barrier block and publish memory init
    if hart::wait_all_harts_finalize(|| {
        mem::physalloc::init(&fdt, mem::vms::virt_to_phys(VAddr::from_ptr(dtb_ptr)));
        mem::vms::remap_tables().expect("failed to remap to higher-half");
        mem::heap::init_heap();
    })
    .is_err()
    {
        panic!("multiple finalizers for physalloc init barrier");
    }

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
        // Safety: Bump region was excluded from the physalloc init usable regions
        unsafe { mem::physalloc::add_region(&reg) };
    }

    // Safety: hid is the current hart ID
    unsafe { arch::interrupt::init_driver(hid, &fdt) };
    arch::interrupt::init();

    let task = task::new_kernel_task(idle).expect("failed to create idle task");
    // Safety: task already has a context
    unsafe { sched::enqueue(task) };

    // Barrier block until all harts enter higher-half space
    if hart::wait_all_harts_finalize(|| {
        log::trace!("synchronized id-unmap");
        mem::vms::uninit_identity_map().expect("failed to uninitialize identity map");
    })
    .is_err()
    {
        panic!("multiple finalizers for id-unmap barrier");
    }

    log::info!("boot start scheduler hid={}", hid);
    hart::wait_all_harts(); // Wait until all harts are ready to enter the scheduler

    // Safety: Not in a critical section
    unsafe { sched::start() };
}

/// # Safety
/// `hid` must be equal to the executing hart ID.
#[unsafe(no_mangle)]
unsafe extern "C" fn smp_entry(hid: usize, sp: PAddr) -> ! {
    log::info!("SMP hart pentry hid={}", hid);

    let trap_stack = mem::physalloc::alloc(2).expect("failed to allocate trap stack");
    let tsp = VAddr::identity(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init();

    mem::vms::smp_enable_initialized().expect("failed to load initialized address space");
    hart::wait_all_harts(); // Until all SMP harts are started

    // Safety: The dtb_ptr argument is not used by the virtual entry
    unsafe { jump_higher_half(smp_entry_virt as *const (), hid, core::ptr::null(), sp) }
}

unsafe fn smp_entry_virt(hid: usize) -> ! {
    log::info!("SMP hart ventry hid={}", hid);

    let trap_stack = mem::physalloc::alloc(32).expect("failed to allocate trap stack");
    let tsp = mem::vms::phys_to_kernel(trap_stack.end_addr());
    core::mem::forget(trap_stack);
    // Safety: tsp points to the top of trap_stack, which is mapped in bump space
    unsafe { hloc::load_init(hid, tsp) };
    arch::init_higher_half();
    hart::wait_all_harts(); // Until the boot hart initializes physalloc

    log::info!("SMP hart finished init hid={}", hid);
    hart::wait_all_harts(); // Until all harts enter higher-half space
    mem::vms::flush_kernel_address_space().expect("failed to flush kernel TLB cache");
    arch::interrupt::init();

    log::info!("SMP start scheduler hid={}", hid);
    hart::wait_all_harts(); // Wait until all harts are ready to enter the scheduler

    // Safety: Not in a critical section
    unsafe { sched::start() };
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
