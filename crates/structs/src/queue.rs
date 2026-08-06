use core::{
    cell::UnsafeCell, fmt::Debug, mem::MaybeUninit, sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct BoundedQueueSlot<T> {
    seq: AtomicUsize,
    val: UnsafeCell<MaybeUninit<T>>,
}

impl<T> BoundedQueueSlot<T> {
    const fn new(seq: usize) -> Self {
        Self {
            seq: AtomicUsize::new(seq),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

/// A multi-producer, multi-consumer implementation of a Vyukov bounded queue.
#[derive(Debug)]
pub struct BoundedQueue<T, const N: usize> {
    enqueue_rsv: AtomicUsize,
    dequeue_rsv: AtomicUsize,
    buf: [BoundedQueueSlot<T>; N],
}

impl<T, const N: usize> BoundedQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            enqueue_rsv: AtomicUsize::new(0),
            dequeue_rsv: AtomicUsize::new(0),
            buf: core::array::from_fn(BoundedQueueSlot::new),
        }
    }

    /// Returns [Ok] if `value` is successfully inserted. Otherwise, the original value is returned.
    pub fn enqueue(&self, value: T) -> Result<(), T> {
        let mut rsv = self.enqueue_rsv.load(Ordering::Relaxed);
        let mut slot;
        loop {
            slot = &self.buf[rsv % N];
            let seq = slot.seq.load(Ordering::Acquire);
            let diff = seq as isize - rsv as isize;
            if diff == 0 {
                // Slot is free to use: try reserving
                match self.enqueue_rsv.compare_exchange_weak(
                    rsv,
                    rsv + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(rsv_actual) => rsv = rsv_actual,
                }
            } else if diff < 0 {
                // Slot is waiting for a consumer: the queue is full
                return Err(value);
            } else {
                // Slot was already reserved by another producer: fetch a new reservation
                rsv = self.enqueue_rsv.load(Ordering::Relaxed);
            }
        }

        // Safety: This slot is guaranteed to only be exclusively accessed by this thread
        unsafe { slot.val.get().write(MaybeUninit::new(value)); }
        slot.seq.store(rsv + 1, Ordering::Release);
        Ok(())
    }

    /// Returns the dequeued item if it exists, otherwise returns [None].
    pub fn dequeue(&self) -> Option<T> {
        let mut rsv = self.dequeue_rsv.load(Ordering::Relaxed);
        let mut slot;
        loop {
            slot = &self.buf[rsv % N];
            let seq = slot.seq.load(Ordering::Acquire);
            let diff = seq as isize - (rsv + 1) as isize;
            if diff == 0 {
                // Slot is ready to be read: try reserving
                match self.dequeue_rsv.compare_exchange_weak(
                    rsv,
                    rsv + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(rsv_actual) => rsv = rsv_actual,
                }
            } else if diff < 0 {
                // Slot is waiting for a producer: the queue is empty
                return None
            } else {
                // Slot was already reserved by another consumer: fetch a new reservation
                rsv = self.dequeue_rsv.load(Ordering::Relaxed);
            }
        }

        // Safety: This slot is guaranteed to only be exclusively accessed by this thread
        let value = unsafe { slot.val.get().read() };
        // Safety: Since this slot has been written to by a producer, the contents of the slot is initialized
        let value = unsafe { value.assume_init() };
        slot.seq.store(rsv + N + 1, Ordering::Release);
        Some(value)
    }
}

impl<T, const N: usize> Drop for BoundedQueue<T, N> {
    fn drop(&mut self) {
        if !core::mem::needs_drop::<T>() {
            return;
        }

        let head = self.dequeue_rsv.load(Ordering::Relaxed);
        let tail = self.enqueue_rsv.load(Ordering::Relaxed);
        for pos in head..tail {
            let slot = &self.buf[pos % N];
            // Safety: Since every slot before enqueue_rsv has been enqueued and every slot starting from dequeue_rsv
            // has not been dequeued, all the slots in between contains initialized data and can be safely dropped
            unsafe {
                slot.val.get().read().assume_init_drop();
            }
        }
    }
}

unsafe impl<T: Send, const N: usize> Send for BoundedQueue<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for BoundedQueue<T, N> {}
