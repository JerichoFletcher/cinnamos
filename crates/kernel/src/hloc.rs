use core::sync::atomic::AtomicUsize;

use alloc::boxed::Box;

use crate::{
    arch::{self, HartLocal, IrqDisabledSection, Task, VAddr},
    mem::alloc::slab::SlabBox,
};

/// Allows access to the hart-local storage within a critical section.
pub struct HartLocalGuard<'cs> {
    /// Invariant: This pointer is not null and points to an initialized [`HartLocal`] in memory.
    ptr: *mut HartLocal,
    _ms: IrqDisabledSection<'cs>,
}

impl HartLocalGuard<'_> {
    /// Consumes this guard and takes out the [`HartLocal`] pointer.
    ///
    /// While calling this function is safe, dereferencing the pointer is extremely unsafe, since
    /// hart-local storage accesses should be performed within a critical section. For that case,
    /// consider using [`try_with_critical`] instead.
    #[inline]
    pub const fn into_ptr(self) -> *const HartLocal {
        self.ptr
    }

    /// Gets the ID of the hart.
    #[inline]
    pub const fn hid(&self) -> usize {
        // Safety: ptr is safe to dereference
        unsafe { (*self.ptr).hid() }
    }

    /// Gets the task currently being executed by this hart, if any.
    #[inline]
    pub const fn curr_task(&mut self) -> Option<&mut SlabBox<Task>> {
        // Safety: ptr is safe to dereference
        unsafe { (*self.ptr).curr_task() }
    }

    /// Takes the current task out of the hart-local storage.
    /// Subsequent calls of [`curr_task`](Self::curr_task) will return [`None`] until a
    /// new task is assigned.
    #[inline]
    pub fn take_curr_task(&mut self) -> Option<SlabBox<Task>> {
        // Safety: ptr is safe to dereference
        unsafe { (*self.ptr).take_curr_task() }
    }

    /// Sets a new task, returning the previous one if it exists.
    #[inline]
    pub fn set_curr_task(&mut self, task: SlabBox<Task>) -> Option<SlabBox<Task>> {
        // Safety: ptr is safe to dereference
        unsafe { (*self.ptr).set_curr_task(task) }
    }
}

/// Initializes the hart-local storage for this hart and loads it into the thread-pointer
/// register of the hart.
///
/// After calling this function, the created storage can be accessed using [`get`] or [`try_get`].
///
/// # Safety
/// `tsp` must point to the top of a valid stack memory.
#[inline]
pub unsafe fn load_init(hid: usize, tsp: VAddr) {
    log::trace!("init hloc hid={} tsp={:#016x}", hid, tsp);
    arch::load_hart_local(Box::leak(Box::new(HartLocal::new(hid, tsp))));
}

/// Gets an access guard for the hart-local storage from an IRQ-free section.
///
/// # Panic
/// Will panic if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn borrow<'ms>(ms: IrqDisabledSection<'ms>) -> HartLocalGuard<'ms> {
    try_borrow(ms).expect("invalid hart-local pointer")
}

/// Attempts to get an access guard for the hart-local storage from an IRQ-free section.
///
/// Returns [`Err`] if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn try_borrow<'ms>(ms: IrqDisabledSection<'ms>) -> Option<HartLocalGuard<'ms>> {
    let ptr = arch::hart_local();
    if ptr.is_null() || !ptr.is_aligned() {
        None
    } else {
        Some(HartLocalGuard { ptr, _ms: ms })
    }
}

/// Reads the ID for the current hart.
///
/// # Panic
/// Will panic if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn get_hid() -> usize {
    try_get_hid().expect("invalid hart-local pointer")
}

/// Attempts to read the ID for the current hart.
///
/// Returns [`None`] if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn try_get_hid() -> Option<usize> {
    let ptr = arch::hart_local();
    if ptr.is_null() || !ptr.is_aligned() {
        None
    } else {
        // Safety: hid is constant and safe to read
        Some(unsafe { (*ptr).hid() })
    }
}

/// Gets the nesting level of critical sections by this hart.
///
/// # Panic
/// Will panic if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn get_critical_nesting<'a>() -> &'a AtomicUsize {
    try_get_critical_nesting().expect("invalid hart-local pointer")
}

/// Attempts to get the nesting level of critical sections by this hart.
///
/// Returns [`None`] if the hart-local storage has not been initialized for this hart.
#[inline]
pub fn try_get_critical_nesting<'a>() -> Option<&'a AtomicUsize> {
    let ptr = arch::hart_local();
    if ptr.is_null() || !ptr.is_aligned() {
        None
    } else {
        // Safety: critical nesting is an atomic integer that is safe to read
        Some(unsafe { (*ptr).critical_nesting() })
    }
}

/// Gets the hart-local storage pointer from the thread-pointer register of this hart.
///
/// While calling this function is safe, dereferencing the pointer is extremely unsafe, since
/// hart-local storage accesses should be performed within a critical section. For that case,
/// consider using [`try_with_critical`] instead.
#[inline]
pub fn get_ptr() -> *const HartLocal {
    arch::hart_local()
}
