use core::{
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use spin::{RelaxStrategy, Spin};

/// Represents errors that can arise in barrier operations.
#[derive(Debug, Clone, Copy)]
pub enum BarrierError {
    /// Attempted to set a barrier while it is already closed.
    SetWhenClosed,
}

/// A barrier that blocks execution until a certain number of acquirements have been done.
///
/// This synchronization primitive can help to "rendezvous" the execution of a precise number of harts.
/// Be aware that unless you can guarantee that the required number of acquirements can be met, improper
/// usage of this barrier can cause very long or even indefinite execution blocks on acquirers.
#[derive(Debug)]
pub struct Barrier<R: RelaxStrategy = Spin> {
    state: AtomicUsize,
    _phantom: PhantomData<R>,
}

impl<R: RelaxStrategy> Barrier<R> {
    // Semantics for a state:
    // `[(usize::BITS/2)-1  : 0              ]` = count
    // `[(usize::BITS-1)    : (usize::BITS/2)]` = generation
    // `[(usize::BITS)]`                        = finalizer
    const COUNT_BITS: u32 = usize::BITS / 2;
    const GEN_BITS: u32 = usize::BITS / 2 - 1;
    const COUNT_MASK: usize = (1 << Self::COUNT_BITS) - 1;
    const GEN_MASK: usize = (1 << Self::GEN_BITS) - 1;

    /// Creates a new barrier.
    /// The initial state of a barrier is open and does not block any acquirers.
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            _phantom: PhantomData,
        }
    }

    /// If the barrier is currently open, sets this barrier to a requirement of `acquire_num` acquirements
    /// and closes the barrier until the required number of acquirements are claimed.
    ///
    /// `acquire_num` must be non-zero at most equal to the maximum value of half the total bits of `usize`
    /// (e.g. if `usize` is [64-bits](u64) wide, then `acquire_num` is not greater than [`u32::MAX`]).
    ///
    /// If the barrier is currently closed, does nothing and returns [`Err`].
    pub fn set(&self, acquire_num: usize) -> Result<(), BarrierError> {
        assert_ne!(acquire_num, 0);
        assert!(acquire_num <= Self::COUNT_MASK);

        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if Self::count(state) != 0 {
                // Barrier is currently still set
                return Err(BarrierError::SetWhenClosed);
            }
            let gen_num = Self::gen_num(state);
            let new_state = Self::to_state(0, gen_num, acquire_num);

            match self.state.compare_exchange_weak(
                state,
                new_state,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => state = actual,
            }
        }
    }

    /// Claims an acquirement slot in the current barrier generation.
    ///
    /// If the barrier is currently open, the function returns immediately.
    /// If the current claim holds the last acquirement, the function will open the barrier and returns.
    /// Otherwise, execution is blocked until the barrier is opened.
    pub fn acquire(&self) {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            let finalizer = Self::finalizer(state);
            let gen_num = Self::gen_num(state);
            let count = Self::count(state);

            if count == 0 {
                // Barrier is currently open
                return;
            }
            if finalizer != 0 && count == 1 {
                // A finalizer exists for the current generation
                // Wait until the finalizer releases the barrier
                loop {
                    state = self.state.load(Ordering::Acquire);
                    if Self::gen_num(state) != gen_num {
                        return;
                    }
                    R::relax();
                }
            }

            let new_count = count - 1;
            if new_count != 0 {
                let new_state = Self::to_state(0, gen_num, new_count);
                match self.state.compare_exchange_weak(
                    state,
                    new_state,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // Claimed an acquirement, but barrier is not down
                        // When barrier generation changes, the current generation is already open
                        loop {
                            state = self.state.load(Ordering::Acquire);
                            if Self::gen_num(state) != gen_num {
                                return;
                            }
                            R::relax();
                        }
                    }
                    Err(actual) => state = actual,
                }
            } else {
                debug_assert_eq!(finalizer, 0);

                // Claimed the last acquirement: release the barrier
                let next_gen_num = gen_num.wrapping_add(1);
                let new_state = Self::to_state(0, next_gen_num, 0);
                match self.state.compare_exchange_weak(
                    state,
                    new_state,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return,
                    Err(actual) => state = actual,
                }
            }
        }
    }

    /// Attempts to reserve the finalizer for the barrier's current generation.
    ///
    /// If reservation succeeds, the hart will wait until only one acquirement remains, at which point `f`
    /// will be called before releasing the barrier.
    /// Thus, while a reservation is present, no other hart can claim the final acquirement and release
    /// the barrier for the current generation.
    ///
    /// Returns [`Err`] if the finalizer for the current generation is already reserved.
    ///
    /// # Note
    /// Because the barrier is only released after `f` returns, calling `f` must **NEVER** cause a panic.
    /// Otherwise, a barrier can be left in a permanently locked state and indefinitely block all acquirers.
    pub fn try_acquire_finalize<T, F: FnOnce() -> T>(&self, f: F) -> Result<T, F> {
        let mut state = self.state.load(Ordering::Acquire);
        let gen_num = loop {
            let finalizer = Self::finalizer(state);
            let count = Self::count(state);

            if count == 0 {
                // Barrier is currently open
                return Ok(f());
            }
            if finalizer != 0 {
                // The finalizer for this generation has already been reserved
                return Err(f);
            }

            let gen_num = Self::gen_num(state);
            let new_state = Self::to_state(1, gen_num, count);
            match self.state.compare_exchange_weak(
                state,
                new_state,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break gen_num,
                Err(actual) => state = actual,
            }
        };

        loop {
            state = self.state.load(Ordering::Acquire);
            debug_assert_eq!(Self::gen_num(state), gen_num);

            if Self::count(state) == 1 {
                break;
            }
            R::relax();
        }

        let r = f();
        let next_gen_num = gen_num.wrapping_add(1);
        let new_state = Self::to_state(0, next_gen_num, 0);
        self.state.store(new_state, Ordering::Release);

        Ok(r)
    }

    #[inline]
    const fn count(state: usize) -> usize {
        state & Self::COUNT_MASK
    }

    #[inline]
    const fn gen_num(state: usize) -> usize {
        (state >> Self::COUNT_BITS) & Self::GEN_MASK
    }

    #[inline]
    const fn finalizer(state: usize) -> usize {
        (state >> (Self::COUNT_BITS + Self::GEN_BITS)) & 1
    }

    #[inline]
    const fn to_state(finalizer: usize, gen_num: usize, count: usize) -> usize {
        ((finalizer & 1) << (Self::COUNT_BITS + Self::GEN_BITS))
            | ((gen_num & Self::GEN_MASK) << Self::COUNT_BITS)
            | (count & Self::COUNT_MASK)
    }
}

impl<R: RelaxStrategy> Default for Barrier<R> {
    fn default() -> Self {
        Self::new()
    }
}
