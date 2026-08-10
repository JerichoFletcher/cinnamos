use core::num::NonZero;

use crate::arch::addr::PAddr;
pub use crate::arch::virt::PAGE_SIZE;

mod phys;
pub use phys::PhysFrameAlloc;

pub mod virt;

pub mod addrsp;
pub mod alloc;
pub mod heap;
pub mod physalloc;
pub mod vmalloc;
pub mod vms;

/// A physical region in memory with a base address and, optionally, a size.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    base: PAddr,
    size: Option<usize>,
}

impl MemoryRegion {
    /// Creates a new memory region.
    #[inline]
    pub fn new(base: *const u8, size: Option<usize>) -> Self {
        Self {
            base: PAddr::from_ptr(base),
            size,
        }
    }

    /// Gets the base address of the region.
    #[inline]
    pub fn start(&self) -> PAddr {
        self.base
    }

    /// Gets the size of the region, if any.
    #[inline]
    pub fn size(&self) -> Option<usize> {
        self.size
    }

    /// Gets the end address of the region.
    /// If the region has no size, this function returns the base address.
    #[inline]
    pub fn end(&self) -> Option<PAddr> {
        Some(self.base + self.size?)
    }

    /// Gets a pointer to the base of the region.
    #[inline]
    pub fn start_ptr(&self) -> *const u8 {
        self.base.addr() as _
    }
}

/// The result of the subtraction between two sized regions.
#[derive(Debug, Clone, Copy)]
pub enum RegionSubtract {
    /// The subtraction produces no remainder region.
    ///
    /// This result is produced from `A - B` when `B` completely contains `A`.
    None,
    /// The subtraction produces a left remainder region.
    ///
    /// This result is produced from `A - B` when the remainder lies below `B`.
    Left(SizedMemoryRegion),
    /// The subtraction produces a right remainder region.
    ///
    /// This result is produced from `A - B` when the remainder lies above `B`.
    Right(SizedMemoryRegion),
    /// The subtraction produces two remainder regions.
    ///
    /// This result is produced from `A - B` when `A` completely contains `B`,
    /// and the bounds of `B` does not touch the bounds of `A`.
    Both(SizedMemoryRegion, SizedMemoryRegion),
    /// The subtraction is performed between non-overlapping regions.
    ///
    /// This result is produced from `A - B` when `A` does not intersect with `B`.
    Nonoverlapping,
}

/// A physical region in memory with a base address and a non-zero size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizedMemoryRegion {
    base: PAddr,
    size: NonZero<usize>,
}

impl SizedMemoryRegion {
    /// Creates a sized memory region from a given size.
    /// If `size` is [`None`] or `0`, the function returns [`None`].
    #[inline]
    pub fn new(base: PAddr, size: Option<usize>) -> Option<Self> {
        let size = NonZero::new(size?)?;
        Some(Self { base, size })
    }

    /// Creates a sized memory region from a physical address range.
    /// Returns [`None`] if the range does not bound a positive-sized region.
    #[inline]
    pub fn from_range(start: PAddr, end: PAddr) -> Option<Self> {
        Self::new(start, end.addr().checked_sub(start.addr()))
    }

    /// Creates a sized memory region from a raw size.
    ///
    /// # Safety
    /// `size` must be non-zero.
    #[inline]
    pub unsafe fn new_unchecked(base: PAddr, size: usize) -> Self {
        // Safety: size is non-zero
        Self {
            base,
            size: unsafe { NonZero::new_unchecked(size) },
        }
    }

    /// Gets the end address of the region.
    #[inline]
    pub fn end(&self) -> PAddr {
        self.base + self.size.get()
    }

    /// Gets a pointer to the base of the region.
    #[inline]
    pub fn start_ptr(&self) -> *const u8 {
        self.base.addr() as _
    }

    /// Subtracts `other` from this region.
    ///
    /// Specifically, this functions attempts to slice out a subregion within the region that
    /// intersects with `other`. Such an operation may possibly return two, one, or no remainder
    /// regions. See [`RegionSubtract`] for more details.
    pub fn subtract(&self, other: &Self) -> RegionSubtract {
        if self.base < other.base && other.end() < self.end() {
            // Safety: other.base is greater than self.base
            let left = unsafe { Self::new_unchecked(self.base, other.base - self.base) };
            // Safety: self.end is greater than other.end
            let right = unsafe { Self::new_unchecked(other.end(), self.end() - other.end()) };
            RegionSubtract::Both(left, right)
        } else if self.base >= other.base && self.end() <= other.end() {
            RegionSubtract::None
        } else if self.base < other.base && self.end() <= other.end() && self.end() > other.base {
            // Safety: other.base is greater than self.base
            RegionSubtract::Left(unsafe { Self::new_unchecked(self.base, other.base - self.base) })
        } else if self.base >= other.base && self.end() > other.end() && self.base < other.end() {
            // Safety: self.end is greater than other.end
            RegionSubtract::Right(unsafe {
                Self::new_unchecked(other.end(), self.end() - other.end())
            })
        } else {
            RegionSubtract::Nonoverlapping
        }
    }

    /// Returns `true` if this region intersects with `other`.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        (self.base < other.end() && other.base < self.end())
            || (other.base < self.end() && self.base < other.end())
    }
}
