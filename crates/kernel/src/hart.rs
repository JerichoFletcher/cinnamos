use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::barrier::Barrier;

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
/// The caller must ensure that the passed value actually equals to the number of
/// available harts in the system.
#[inline]
pub unsafe fn set_hart_count(count: usize) {
    HART_COUNT.store(count, Ordering::Release);
}

/// Waits until all harts acquire the global barrier.
///
/// This function tries to close the global barrier if it is open, and then immediately acquires it.
/// The number of acquirers to use is determined by the value of `HART_COUNT` (see [`get_hart_count`],
/// [`set_hart_count`]).
#[inline]
pub fn wait_all_harts() {
    let _ = GLOBAL_BARRIER.set(get_hart_count());
    GLOBAL_BARRIER.acquire();
}

/// Waits until all other harts acquire the global barrier, then executes `f` before releasing the barrier.
///
/// If another hart has acquired the barrier with a finalizer, this function does nothing and immediately
/// returns `f` back to the caller in an [`Err`].
///
/// While a finalizer is reserved for this barrier, only the finalizer acquirer can release the barrier.
/// No other acquirer can claim the last acquirement for a generation, and instead they will wait until
/// the finalizer is finished.
///
/// This function tries to close the global barrier if it is open, and then immediately acquires it.
/// The number of acquirers to use is determined by the value of `HART_COUNT` (see [`get_hart_count`],
/// [`set_hart_count`]).
///
/// # Note
/// Because the barrier is only released after `f` returns, calling `f` must **NEVER** cause a panic.
/// Otherwise, a barrier can be left in a permanently locked state and indefinitely block all acquirers.
#[inline]
pub fn wait_all_harts_finalize<T, F: FnOnce() -> T>(f: F) -> Result<T, F> {
    let _ = GLOBAL_BARRIER.set(get_hart_count());
    GLOBAL_BARRIER.try_acquire_finalize(f)
}
