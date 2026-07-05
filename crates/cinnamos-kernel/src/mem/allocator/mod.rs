mod freelist;
pub use freelist::*;

use crate::arch::paddr::PAddr;

pub trait FrameAlloc {
    fn addr(&self) -> PAddr;
}

pub trait PhysFrameAllocator<T : FrameAlloc> {
    fn size(start: PAddr, end: PAddr) -> usize;

    fn alloc(&mut self, size_bytes: usize) -> Option<T>;
    fn dealloc(&mut self, handle: T);
    fn reserve(&mut self, start: PAddr, end: PAddr);
}
