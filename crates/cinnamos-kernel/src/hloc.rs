use core::mem::MaybeUninit;

use alloc::{boxed::Box, vec::Vec};
use spin::Once;

use crate::{
    arch::{self, HartLocal, PAddr, VAddr},
    mem::{PhysFrameAlloc, alloc::slab::SlabBox, physalloc::FrameAlloc},
    task::Task,
};

pub struct HartLocalGuard {
    /// Equals to the pointer in the thread-pointer register for this hart.
    ptr: *mut HartLocal,
}

impl HartLocalGuard {
    #[inline]
    pub const fn as_ptr(&self) -> *const () {
        self.ptr.cast()
    }

    #[inline]
    pub const fn hid(&self) -> usize {
        // Safety: ptr is valid
        unsafe { (*self.ptr).hid() }
    }

    #[inline]
    pub const fn curr_task(&mut self) -> Option<&mut SlabBox<Task>> {
        // Safety: ptr is valid
        unsafe { (*self.ptr).curr_task() }
    }

    #[inline]
    pub fn take_curr_task(&mut self) -> Option<SlabBox<Task>> {
        // Safety: ptr is valid
        unsafe { (*self.ptr).take_curr_task() }
    }

    #[inline]
    pub fn set_curr_task(&mut self, task: SlabBox<Task>) {
        // Safety: ptr is valid
        unsafe {
            (*self.ptr).set_curr_task(task);
        }
    }
}

static mut BOOT_HLOC: MaybeUninit<HartLocal> = MaybeUninit::zeroed();
static HLOCS: Once<&[HartLocal]> = Once::new();

/// Should only be called by the boot hart.
///
/// # Safety
/// - `hid` must be equal to the executing hart ID.
/// - `tsp` must point to the top of a valid stack memory.
#[inline]
pub unsafe fn init_boot_hart_local(hid: usize, tsp: VAddr) {
    let ptr = &raw mut BOOT_HLOC;
    // Safety: ptr points to a HartLocal, and is only accessed by the boot hart
    let ptr = unsafe { ptr.as_mut_unchecked().write(HartLocal::new(hid, tsp)) };
    arch::load_hart_local(ptr);
}

/// Should only be called by the boot hart.
///
/// # Safety
/// `p2v` must be a translation function to a valid address space with respect to the frame allocations
/// provided in `trap_stacks`.
#[inline]
pub unsafe fn init_hlocs(trap_stacks: Box<[FrameAlloc]>, p2v: impl Fn(PAddr) -> VAddr) {
    HLOCS.call_once(|| {
        Box::leak(
            trap_stacks
                .into_iter()
                .enumerate()
                .map(|(hid, tstack)| {
                    let tsp = p2v(tstack.start_addr());
                    core::mem::forget(tstack);
                    log::trace!("init hloc hid={} tsp={:#016x}", hid, tsp);
                    HartLocal::new(hid, tsp)
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    });
}

/// # Safety
/// `hid` must be equal to the executing hart ID.
///
/// # Panic
/// Will panic if the hart-local storage buffer has not been [initialized](init_hlocs),
/// or if the hart ID is invalid.
#[inline]
pub unsafe fn load_hart_local(hid: usize) {
    let buf = HLOCS
        .get()
        .expect("hart local storage has not been initialized");
    let ptr = if hid < buf.len() {
        &buf[hid]
    } else {
        panic!("invalid HID ({} vs. max {})", hid, buf.len())
    };
    arch::load_hart_local(ptr);
}

/// Should only be called after the [HartLocal](HartLocal) for the caller has been loaded.
#[inline]
pub fn hart_local() -> HartLocalGuard {
    let ptr = arch::hart_local();
    if ptr.is_null() || !ptr.is_aligned() {
        panic!("invalid hart-local pointer {:p}", ptr);
    }
    HartLocalGuard { ptr }
}
