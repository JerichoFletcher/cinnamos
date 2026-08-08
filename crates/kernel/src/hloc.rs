use alloc::boxed::Box;

use crate::{
    arch::{self, HartLocal, Task, VAddr},
    mem::alloc::slab::SlabBox,
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
        unsafe { (*self.ptr).set_curr_task(task) };
    }
}

/// # Safety
/// `tsp` must point to the top of a valid stack memory.
#[inline]
pub unsafe fn load_init(hid: usize, tsp: VAddr) {
    log::trace!("init hloc hid={} tsp={:#016x}", hid, tsp);
    arch::load_hart_local(Box::leak(Box::new(HartLocal::new(hid, tsp))));
}

/// Should only be called after the [HartLocal](HartLocal) for the caller has been loaded.
#[inline]
pub fn get() -> HartLocalGuard {
    let ptr = arch::hart_local();
    if ptr.is_null() || !ptr.is_aligned() {
        panic!("invalid hart-local pointer {:p}", ptr);
    }
    HartLocalGuard { ptr }
}
