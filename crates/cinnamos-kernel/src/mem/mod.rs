pub use crate::arch::PAGE_SIZE;
use crate::arch::PAddr;

mod phys;
pub use phys::FrameAlloc;

pub mod palloc;
// pub mod valloc;
pub mod heap;
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
            size: size,
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
