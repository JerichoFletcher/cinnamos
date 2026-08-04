use core::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use spin::mutex::{Mutex, MutexGuard};

use crate::arch::IrqState;

pub struct MutexIrqSave<T: ?Sized, R = spin::Spin> {
    inner: Mutex<T, R>,
}

impl<T, R> MutexIrqSave<T, R> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }
}

impl<T: ?Sized, R: spin::RelaxStrategy> MutexIrqSave<T, R> {
    #[inline(always)]
    pub fn lock(&self) -> MutexIrqSaveGuard<'_, T, R> {
        let irq = IrqState::save_disable();
        let inner = self.inner.lock();
        MutexIrqSaveGuard {
            inner: ManuallyDrop::new(inner),
            irq: Some(irq),
        }
    }
}

unsafe impl<T: ?Sized, R> Send for MutexIrqSave<T, R> {}
unsafe impl<T: ?Sized, R> Sync for MutexIrqSave<T, R> {}

pub struct MutexIrqSaveGuard<'a, T: ?Sized + 'a, R = spin::Spin> {
    inner: ManuallyDrop<MutexGuard<'a, T, R>>,
    irq: Option<IrqState>,
}

impl<'a, T: ?Sized, R> Drop for MutexIrqSaveGuard<'a, T, R> {
    fn drop(&mut self) {
        // Safety: The owner guard is dropped after finishing this
        unsafe {
            ManuallyDrop::drop(&mut self.inner);
        }
        if let Some(irq) = self.irq.take() {
            irq.restore();
        }
    }
}

impl<'a, T: ?Sized, R> Deref for MutexIrqSaveGuard<'a, T, R> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T: ?Sized, R> DerefMut for MutexIrqSaveGuard<'a, T, R> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

unsafe impl<T: ?Sized, R> Sync for MutexIrqSaveGuard<'_, T, R> where for<'a> &'a mut T: Sync {}
unsafe impl<T: ?Sized, R> Send for MutexIrqSaveGuard<'_, T, R> where for<'a> &'a mut T: Send {}
