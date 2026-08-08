use core::{
    fmt::{Debug, LowerHex},
    ops::{Add, Sub},
    ptr::NonNull,
};

use crate::{
    arch::{PAddr, sv48::PT_MAX_ENTRIES},
    mem::PAGE_SIZE,
};

/// Represents a virtual address.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(usize);

impl VAddr {
    /// A null virtual address.
    pub const NULL: Self = Self(0);

    /// Creates a new virtual address from raw unsigned integer.
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Creates an identity-mapped virtual address from a physical address.
    #[inline]
    pub const fn identity(paddr: PAddr) -> Self {
        Self(paddr.addr())
    }

    /// Creates a virtual address from known virtual page numbers, as well as an offset.
    #[inline]
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

    /// Creates a virtual address from a pointer.
    #[inline]
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self(ptr as *const () as usize)
    }

    /// Creates a pointer from this virtual address.
    #[inline]
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    /// Creates a mutable pointer from this virtual address.
    #[inline]
    pub const fn as_mut<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    /// Creates a [NonNull] from this virtual address.
    #[inline]
    pub const fn as_nonnull<T>(&self) -> Option<NonNull<T>> {
        NonNull::new(self.0 as *mut T)
    }

    /// Gets the unsigned integer address of this virtual address.
    #[inline]
    pub const fn addr(&self) -> usize {
        self.0
    }

    /// Gets the virtual page numbers of the virtual page containing this address.
    #[inline]
    pub const fn vpn(&self) -> [usize; 4] {
        [
            (self.0 >> 12) & 0x1ff,
            (self.0 >> 21) & 0x1ff,
            (self.0 >> 30) & 0x1ff,
            (self.0 >> 39) & 0x1ff,
        ]
    }

    /// Gets the aggregated virtual page number of the virtual page containing this address.
    #[inline]
    pub const fn vpn_all(&self) -> usize {
        (self.0 / PAGE_SIZE) & ((1 << 36) - 1)
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

    /// Reads the value from the memory location at this address.
    /// Does not modify or move the old value.
    ///
    /// # Safety
    /// The address must be readable and aligned to `T` (see [`ptr::write`](core::ptr::write)).
    #[inline]
    pub unsafe fn read<T>(&self) -> T {
        unsafe { self.as_ptr::<T>().read() }
    }

    /// Overwrites the memory location at this address with the given value.
    /// Does not read or drop the old value at the location.
    ///
    /// # Safety
    /// The address must be writable and aligned to `T` (see [`ptr::write`](core::ptr::write)).
    #[inline]
    pub unsafe fn write<T>(&self, val: T) {
        unsafe { self.as_mut::<T>().write(val); }
    }
}

impl Add<usize> for VAddr {
    type Output = VAddr;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl Sub<usize> for VAddr {
    type Output = VAddr;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl Sub<VAddr> for VAddr {
    type Output = usize;

    fn sub(self, rhs: VAddr) -> Self::Output {
        self.0.wrapping_sub(rhs.0)
    }
}

impl LowerHex for VAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Debug for VAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VAddr({:#016x})", self.0)
    }
}
