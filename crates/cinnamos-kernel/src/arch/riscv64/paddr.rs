use core::ops::{Add, Sub};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PAddr(usize);

impl PAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn from_ptr(ptr: *const u8) -> Self {
        Self(ptr as usize)
    }

    pub fn addr(&self) -> usize {
        self.0
    }
}

impl Add<usize> for PAddr {
    type Output = PAddr;

    fn add(self, rhs: usize) -> Self::Output {
        PAddr(self.0 + rhs)
    }
}

impl Sub<usize> for PAddr {
    type Output = PAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        PAddr(self.0 - rhs)
    }
}

impl Sub<PAddr> for PAddr {
    type Output = usize;

    fn sub(self, rhs: PAddr) -> Self::Output {
        self.0 - rhs.0
    }
}
