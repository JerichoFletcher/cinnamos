use crate::arch::paddr::PAddr;

mod allocator;

pub mod palloc;
pub use allocator::FrameAlloc;

pub const PAGE_SIZE: usize = 4096;

pub struct MemoryRegion {
    base: PAddr,
    size: Option<usize>,
}

impl MemoryRegion {
    pub fn new(base: *const u8, size: Option<usize>) -> Self {
        Self { base: PAddr::from_ptr(base), size: size }
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
