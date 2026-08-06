use crate::arch::VAddr;

#[derive(Debug)]
pub struct StackBuilder {
    ptr: VAddr,
}

impl StackBuilder {
    #[inline]
    pub const fn new(at: VAddr) -> Self {
        Self { ptr: at }
    }

    #[inline]
    pub fn get(&self) -> VAddr {
        self.ptr
    }

    /// # Safety
    /// Given the current stack pointer as `ptr`:
    /// - `ptr` must point to writable memory.
    /// - There must be enough space below `ptr` to fit an aligned instance of `T`.
    pub unsafe fn push<T>(&mut self, val: T) -> &mut Self {
        self.ptr = (self.ptr - size_of::<T>()).align_down(align_of::<T>());
        // Safety: ptr is aligned to T
        unsafe {
            self.ptr.write(val);
        }
        self
    }
}
