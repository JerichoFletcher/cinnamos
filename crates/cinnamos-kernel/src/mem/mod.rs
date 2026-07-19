pub use crate::arch::PAGE_SIZE;
use crate::arch::PAddr;

mod phys;
pub use phys::PhysFrameAlloc;

pub mod bump;
pub mod heap;
pub mod palloc;
// pub mod valloc;
pub mod vms;

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    base: PAddr,
    size: Option<usize>,
}

impl MemoryRegion {
    pub fn new(base: *const u8, size: Option<usize>) -> Self {
        Self {
            base: PAddr::from_ptr(base),
            size,
        }
    }

    pub fn start(&self) -> PAddr {
        self.base
    }

    pub fn size(&self) -> Option<usize> {
        self.size
    }

    pub fn end(&self) -> Option<PAddr> {
        Some(self.base + self.size?)
    }

    pub fn start_ptr(&self) -> *const u8 {
        core::ptr::without_provenance(self.base.addr())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RegionSubtract {
    None,
    Left(SizedMemoryRegion),
    Right(SizedMemoryRegion),
    Both(SizedMemoryRegion, SizedMemoryRegion),
    Nonoverlapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizedMemoryRegion {
    base: PAddr,
    size: usize,
}

impl SizedMemoryRegion {
    pub fn new(base: PAddr, size: Option<usize>) -> Option<Self> {
        match size {
            Some(size) => {
                if size > 0 {
                    Some(Self { base, size })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// # Safety
    /// `size` must be non-zero.
    pub unsafe fn new_unchecked(base: PAddr, size: usize) -> Self {
        Self { base, size }
    }

    pub fn end(&self) -> PAddr {
        self.base + self.size
    }

    pub fn start_ptr(&self) -> *const u8 {
        core::ptr::without_provenance(self.base.addr())
    }

    pub fn subtract(&self, other: &Self) -> RegionSubtract {
        if self.base < other.base && other.end() < self.end() {
            let left = Self {
                base: self.base,
                size: other.base - self.base,
            };
            let right = Self {
                base: other.end(),
                size: self.end() - other.end(),
            };
            RegionSubtract::Both(left, right)
        } else if self.base >= other.base && self.end() <= other.end() {
            RegionSubtract::None
        } else if self.base < other.base && self.end() <= other.end() && self.end() > other.base {
            RegionSubtract::Left(Self {
                base: self.base,
                size: other.base - self.base,
            })
        } else if self.base >= other.base && self.end() > other.end() && self.base < other.end() {
            RegionSubtract::Right(Self {
                base: other.end(),
                size: self.end() - other.end(),
            })
        } else {
            RegionSubtract::Nonoverlapping
        }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        (self.base < other.end() && other.base < self.end())
            || (other.base < self.end() && self.base < other.end())
    }
}
