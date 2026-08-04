pub mod buddy;

use crate::{arch::VAddr, mem::PAGE_SIZE};

pub trait VirtAlloc {
    fn start_addr(&self) -> VAddr;
    fn end_addr(&self) -> VAddr;

    fn size(&self) -> usize {
        self.end_addr() - self.start_addr()
    }

    fn page_count(&self) -> usize {
        self.size() / PAGE_SIZE
    }
}

pub trait VirtAllocator<T: VirtAlloc> {
    fn alloc(&self, page_count: usize) -> Option<T>;
    fn alloc_guarded(&self, page_count: usize, guard_page_count: usize) -> Option<T>;
    fn dealloc(&self, handle: &mut T);
}
