mod freelist;
pub use freelist::FreeListAllocator as FrameAllocator;
pub use freelist::FreeListFrameAlloc as FrameAllocation;

use crate::arch::PAddr;

pub trait FrameAlloc {
    fn base_addr(&self) -> PAddr;
}

pub trait PhysFrameAllocator<T : FrameAlloc> {
    fn size(start: PAddr, end: PAddr) -> usize;

    fn alloc(&mut self, size_bytes: usize) -> Option<T>;
    fn dealloc(&mut self, handle: &mut T);
    fn reserve(&mut self, start: PAddr, end: PAddr);
}
