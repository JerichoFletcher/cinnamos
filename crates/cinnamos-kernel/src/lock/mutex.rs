use core::sync::atomic::{AtomicBool, Ordering};
use lock_api::{GuardSend, RawMutex};

pub struct RawSpinLock {
    lock: AtomicBool,
}

unsafe impl RawMutex for RawSpinLock {
    const INIT: Self = Self { lock: AtomicBool::new(false) };
    type GuardMarker = GuardSend;

    fn lock(&self) {
        while !self.try_lock() {}
    }

    fn try_lock(&self) -> bool {
        match self.lock.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
            Ok(..) => true,
            Err(..) => false,
        }
    }

    unsafe fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

pub type SpinLock<T> = lock_api::Mutex<RawSpinLock, T>;
pub type SpinLockGuard<'a, T> = lock_api::MutexGuard<'a, RawSpinLock, T>;
