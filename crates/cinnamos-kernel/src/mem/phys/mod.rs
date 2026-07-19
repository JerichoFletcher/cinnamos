pub mod buddy;

use crate::arch::PAddr;

pub trait PhysFrameAlloc {
    fn base_addr(&self) -> PAddr;
}

pub trait PhysFrameAllocator<T: PhysFrameAlloc> {
    fn alloc(&mut self, frame_count: usize) -> Option<T>;
    fn dealloc(&mut self, handle: &mut T);
}
