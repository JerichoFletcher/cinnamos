use core::fmt::LowerHex;

use crate::arch::{PAddr, sv48::PT_MAX_ENTRIES};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(usize);

impl VAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn identity(paddr: PAddr) -> Self {
        Self(paddr.addr())
    }

    pub fn from_parts(vpn: [usize; 4], page_offset: usize) -> Self {
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

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub fn addr(&self) -> usize {
        self.0
    }

    pub fn vpn(&self) -> [usize; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1ff,
        ]
    }
}

impl LowerHex for VAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}
