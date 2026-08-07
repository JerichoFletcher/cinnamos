use core::{
    fmt::{Debug, LowerHex},
    ops::{Add, Sub},
};

use crate::mem::PAGE_SIZE;

/// Represents a physical address. A pointer should not be safely derived from this address directly
/// unless it has been made sure that the resulting pointer is properly mapped.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PAddr(usize);

impl PAddr {
    /// A null physical address.
    pub const NULL: Self = Self(0);

    /// Creates a new physical address from raw unsigned integer.
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Creates a physical address from a pointer.
    #[inline]
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self(ptr as *const () as usize)
    }

    /// Gets the unsigned integer address of this physical address.
    #[inline]
    pub const fn addr(&self) -> usize {
        self.0
    }

    /// Gets the aggregated physical page number of the physical page containing this address.
    #[inline]
    pub const fn ppn_all(&self) -> usize {
        (self.0 / PAGE_SIZE) & ((1 << 44) - 1)
    }

    /// Aligns down to the first aligned address less than equal than the current address.
    ///
    /// Only correct if `align` is a [power of two](usize::is_power_of_two).
    #[inline]
    pub const fn align_down(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self(self.0 & !(align - 1))
    }

    /// Aligns up to the first aligned address greater or equal than the current address.
    ///
    /// Only correct if `align` is a [power of two](usize::is_power_of_two).
    #[inline]
    pub const fn align_up(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Aligns the address to its page.
    #[inline]
    pub const fn align_to_page(&self) -> Self {
        self.align_down(PAGE_SIZE)
    }

    /// Aligns the address to the next page.
    #[inline]
    pub const fn align_to_next_page(&self) -> Self {
        self.align_up(PAGE_SIZE)
    }
}

impl Add<usize> for PAddr {
    type Output = PAddr;

    fn add(self, rhs: usize) -> Self::Output {
        PAddr(self.0.wrapping_add(rhs))
    }
}

impl Sub<usize> for PAddr {
    type Output = PAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        PAddr(self.0.wrapping_sub(rhs))
    }
}

impl Sub<PAddr> for PAddr {
    type Output = usize;

    fn sub(self, rhs: PAddr) -> Self::Output {
        self.0.wrapping_sub(rhs.0)
    }
}

impl LowerHex for PAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Debug for PAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PAddr({:#016x})", self.0)
    }
}
