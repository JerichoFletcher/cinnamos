use core::{fmt::{Debug, LowerHex}, ops::{Add, Sub}};

use crate::mem::PAGE_SIZE;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PAddr(usize);

impl PAddr {
    pub const NULL: Self = Self(0);

    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as usize)
    }

    pub const fn addr(&self) -> usize {
        self.0
    }

    pub const fn ppn(&self) -> usize {
        self.0 / PAGE_SIZE
    }

    pub const fn next_multiple_of(&self, rhs: usize) -> Self {
        Self::new(self.addr().next_multiple_of(rhs))
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
        write!(f, "PAddr(0x{:016x})", self.0)
    }
}
