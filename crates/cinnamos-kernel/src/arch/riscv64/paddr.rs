use core::{
    fmt::{Debug, LowerHex},
    ops::{Add, Sub},
};

use crate::mem::PAGE_SIZE;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PAddr(usize);

impl PAddr {
    pub const NULL: Self = Self(0);

    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as usize)
    }

    #[inline]
    pub const fn addr(&self) -> usize {
        self.0
    }

    #[inline]
    pub const fn ppn_all(&self) -> usize {
        (self.0 / PAGE_SIZE) & ((1 << 44) - 1)
    }

    /// Only correct if `align` is a [power of two](usize::is_power_of_two).
    #[inline]
    pub const fn align_down(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self(self.0 & !(align - 1))
    }

    /// Only correct if `align` is a [power of two](usize::is_power_of_two).
    #[inline]
    pub const fn align_up(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        Self((self.0 + align - 1) & !(align - 1))
    }

    #[inline]
    pub const fn align_to_page(&self) -> Self {
        self.align_down(PAGE_SIZE)
    }

    #[inline]
    pub const fn align_to_next_page(&self) -> Self {
        self.align_up(PAGE_SIZE)
    }

    #[inline]
    pub const fn next_multiple_of(&self, rhs: usize) -> Self {
        Self::new(self.0.next_multiple_of(rhs))
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
