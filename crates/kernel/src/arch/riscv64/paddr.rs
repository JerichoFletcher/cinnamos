use core::{
    fmt::{Debug, LowerHex},
    ops::{Add, Sub},
};

use sbi::PhysicalAddress;

use crate::{
    arch::{addr::VAddr, sv48::PT_MAX_ENTRIES, virt::PageLevel},
    mem::PAGE_SIZE,
};

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

    /// Creates a physical address from known physical page numbers, as well as an offset.
    #[inline]
    pub const fn from_parts(ppn: [usize; 4], page_offset: usize) -> Self {
        debug_assert!(ppn[0] < PT_MAX_ENTRIES);
        debug_assert!(ppn[1] < PT_MAX_ENTRIES);
        debug_assert!(ppn[2] < PT_MAX_ENTRIES);
        debug_assert!(ppn[3] < PT_MAX_ENTRIES);

        let ppn0 = (ppn[0] & 0x1ff) << 12;
        let ppn1 = (ppn[1] & 0x1ff) << 21;
        let ppn2 = (ppn[2] & 0x1ff) << 30;
        let ppn3 = (ppn[3] & 0x1ff) << 39;
        let page_offset = page_offset & 0xfff;

        Self(ppn3 | ppn2 | ppn1 | ppn0 | page_offset)
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

    /// Gets the physical page numbers of the physical page containing this address.
    #[inline]
    pub const fn ppn(&self) -> [usize; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1ffff,
        ]
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

    /// Computes the effective physical address from the combination of the base physical address
    /// and the effective offset, which is computed from the given virtual address and the [`PageLevel`]
    /// at which the cutoff should happen.
    pub const fn compute_phys(&self, va: &VAddr, level: Option<PageLevel>) -> Self {
        let offset = va.addr() % PAGE_SIZE;
        let [ppn0, ppn1, ppn2, ppn3] = self.ppn();
        let [vpn0, vpn1, vpn2, vpn3] = va.vpn();
        let ppns = match level {
            None => [ppn0, ppn1, ppn2, ppn3],
            Some(PageLevel::Page4K) => [vpn0, ppn1, ppn2, ppn3],
            Some(PageLevel::Megapage2M) => [vpn0, vpn1, ppn2, ppn3],
            Some(PageLevel::Gigapage1G) => [vpn0, vpn1, vpn2, ppn3],
            Some(PageLevel::Terapage512G) => [vpn0, vpn1, vpn2, vpn3],
        };
        Self::from_parts(ppns, offset)
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

impl<T> From<PAddr> for PhysicalAddress<T> {
    fn from(value: PAddr) -> Self {
        Self::new(value.0)
    }
}
