pub mod buddy;

use crate::arch::VAddr;

pub trait VirtAlloc {
    fn start_addr(&self) -> VAddr;
    fn end_addr(&self) -> VAddr;

    fn size(&self) -> usize {
        self.end_addr() - self.start_addr()
    }
}

pub trait VirtAllocator<T: VirtAlloc> {
    fn alloc(&self, page_count: usize) -> Option<T>;
    fn dealloc(&self, handle: &mut T);
}
