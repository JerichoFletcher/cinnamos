pub mod buddy;

use crate::arch::PAddr;

pub trait PhysFrameAlloc {
    fn start_addr(&self) -> PAddr;
    fn end_addr(&self) -> PAddr;

    fn size(&self) -> usize {
        self.end_addr() - self.start_addr()
    }
}

pub trait PhysFrameAllocator<T: PhysFrameAlloc> {
    fn alloc(&mut self, frame_count: usize) -> Option<T>;
    fn dealloc(&mut self, handle: &mut T);
}
