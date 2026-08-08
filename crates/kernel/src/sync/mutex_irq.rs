use spin::{Mutex, MutexGuard};

use crate::arch::IrqDisabledSection;

/// A variation of spin mutex that can only be acquired from an IRQ-free section.
#[derive(Debug)]
pub struct MutexIrq<T: ?Sized> {
    inner: Mutex<T>,
}

impl<T> MutexIrq<T> {
    /// Creates a new [`MutexIrq`] wrapping the given value.
    pub const fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }
}

impl<T: ?Sized> MutexIrq<T> {
    /// Acquires a lock on the [`MutexIrq`] and returns an access guard to the inner data.
    ///
    /// Requires a token of proof that the lock is acquired from an IRQ-free section.
    pub fn lock<'ms>(&'ms self, _ms: IrqDisabledSection<'ms>) -> MutexGuard<'ms, T> {
        self.inner.lock()
    }
}
