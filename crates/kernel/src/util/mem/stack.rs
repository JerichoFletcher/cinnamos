use crate::arch::addr::VAddr;

/// A utility for creating an in-memory stack.
#[derive(Debug)]
pub struct StackBuilder {
    ptr: VAddr,
}

impl StackBuilder {
    /// Creates a new [`StackBuilder`] with the given starting pointer.
    #[inline]
    pub const fn new(at: VAddr) -> Self {
        Self { ptr: at }
    }

    /// Gets the current stack pointer.
    #[inline]
    pub fn get(&self) -> VAddr {
        self.ptr
    }

    /// Pushes a value into the stack and shifts down the stack pointer.
    ///
    /// The stack pointer is shifted down such that the new pointer is aligned to `T`.
    /// As such, the value is considered safely initialized within the stack memory.
    ///
    /// The function re-returns the mutable `self` reference, which allows chaining
    /// multiple [`push`](Self::push) calls together.
    ///
    /// # Safety
    /// Given the current stack pointer as `ptr`:
    /// - `ptr` must point to writable memory.
    /// - There must be enough space below `ptr` to fit an aligned instance of `T`.
    pub unsafe fn push<T>(&mut self, val: T) -> &mut Self {
        self.ptr = (self.ptr - size_of::<T>()).align_down(align_of::<T>());
        // Safety: ptr is aligned to T
        unsafe { self.ptr.write(val) };
        self
    }
}
