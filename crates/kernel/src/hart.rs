use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::barrier::{Barrier, BarrierError};

static HART_COUNT: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_BARRIER: Barrier = Barrier::new();

/// Reads the `HART_COUNT` global variable.
#[inline]
pub fn get_hart_count() -> usize {
    HART_COUNT.load(Ordering::Acquire)
}

/// Sets the `HART_COUNT` global variable.
///
/// # Safety
/// The caller must make sure that the passed value actually equals to the number of
/// available harts in the system.
#[inline]
pub unsafe fn set_hart_count(count: usize) {
    HART_COUNT.store(count, Ordering::Release);
}

/// Waits until all harts acquire the global barrier.
///
/// This function tries to close the global barrier if it is open, and then
/// immediately acquires it.
/// The number of acquirers to use is determined by the value of `HART_COUNT`
/// (see [`get_hart_count`], [`set_hart_count`]).
#[inline]
pub fn wait_all_harts() {
    let _ = set_barrier_all_harts();
    acquire_barrier();
}

/// Waits until all other harts acquire the global barrier, then executes
/// `f` before releasing the barrier.
///
/// If another hart has acquired the barrier with a finalizer, this function
/// does not call `f` or acquire the barrier. Instead, it returns `f`
/// back to the caller in an [`Err`].
///
/// While a finalizer is waiting for this barrier, only the finalizer acquirer
/// can release the barrier. No other acquirer can claim the last acquirement
/// for a generation, and instead they will wait until the finalizer is
/// finished.
///
/// This function tries to close the global barrier if it is open, and then
/// immediately acquires it.
/// The number of acquirers to use is determined by the value of `HART_COUNT`
/// (see [`get_hart_count`], [`set_hart_count`]).
///
/// # Note
/// Because the barrier is only released after `f` returns, calling `f` must
/// **NEVER** cause a panic. Otherwise, a barrier can be left in a permanently
/// locked state and indefinitely block all acquirers.
#[inline]
pub fn wait_all_harts_finalize<T, F: FnOnce() -> T>(_f: F) -> Result<T, F> {
    let _ = set_barrier_all_harts();
    todo!("implement finalizer")
}

/// Sets the global barrier to close until all available harts have acquired it.
///
/// The number of acquirers to use is determined by the value of `HART_COUNT`
/// (see [`get_hart_count`], [`set_hart_count`]).
#[inline]
fn set_barrier_all_harts() -> Result<(), BarrierError> {
    GLOBAL_BARRIER.set(get_hart_count())
}

/// Acquires the global barrier and waits until enough harts have acquired it as well.
#[inline]
fn acquire_barrier() {
    GLOBAL_BARRIER.acquire();
}
