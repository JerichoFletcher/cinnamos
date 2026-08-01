pub mod buddy;

use crate::{arch::PAddr, mem::PAGE_SIZE};

pub trait PhysFrameAlloc {
    fn start_addr(&self) -> PAddr;
    fn end_addr(&self) -> PAddr;

    fn frame_count(&self) -> usize {
        self.size() / PAGE_SIZE
    }

    fn size(&self) -> usize {
        self.end_addr() - self.start_addr()
    }
}

pub trait PhysFrameAllocator<T: PhysFrameAlloc> {
    fn alloc(&self, frame_count: usize) -> Option<T>;
    fn dealloc(&self, handle: &mut T);
}
