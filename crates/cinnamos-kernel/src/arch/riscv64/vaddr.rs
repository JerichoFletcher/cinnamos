use core::{fmt::{Debug, LowerHex}, ops::{Add, Sub}};

use crate::arch::{PAddr, sv48::PT_MAX_ENTRIES};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(usize);

impl VAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn identity(paddr: PAddr) -> Self {
        Self(paddr.addr())
    }

    pub const fn from_parts(vpn: [usize; 4], page_offset: usize) -> Self {
        debug_assert!(vpn[0] < PT_MAX_ENTRIES);
        debug_assert!(vpn[1] < PT_MAX_ENTRIES);
        debug_assert!(vpn[2] < PT_MAX_ENTRIES);
        debug_assert!(vpn[3] < PT_MAX_ENTRIES);

        let vpn0 = (vpn[0] & 0x1ff) << 12;
        let vpn1 = (vpn[1] & 0x1ff) << 21;
        let vpn2 = (vpn[2] & 0x1ff) << 30;
        let vpn3 = (vpn[3] & 0x1ff) << 39;
        let page_offset = page_offset & 0xfff;

        Self(vpn3 | vpn2 | vpn1 | vpn0 | page_offset)
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as usize)
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub const fn as_mut<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub const fn addr(&self) -> usize {
        self.0
    }

    pub const fn vpn(&self) -> [usize; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1ff,
        ]
    }
}

impl Add<usize> for VAddr {
    type Output = VAddr;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for VAddr {
    type Output = VAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<VAddr> for VAddr {
    type Output = usize;

    fn sub(self, rhs: VAddr) -> Self::Output {
        self.0 - rhs.0
    }
}

impl LowerHex for VAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Debug for VAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VAddr(0x{:016x})", self.0)
    }
}
