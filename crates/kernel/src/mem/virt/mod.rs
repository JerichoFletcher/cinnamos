pub mod buddy;

use core::num::NonZero;

use crate::arch::addr::VAddr;

/// A trait for virtual page allocations.
pub trait VirtAlloc {
    /// The base address of the allocated virtual region.
    fn start_addr(&self) -> VAddr;

    /// The end address of the allocated virtual region.
    fn end_addr(&self) -> VAddr;

    /// The number of virtual pages reserved for this allocation.
    fn page_count(&self) -> NonZero<usize>;

    /// The size of the allocated region, in bytes.
    fn size(&self) -> usize;
}

pub trait VirtAllocator<T: VirtAlloc> {
    /// Requests an allocation of virtual pages.
    ///
    /// Returns [`None`] if allocation fails or attempts to create a zero-page allocation.
    fn alloc(&self, page_count: usize) -> Option<T>;

    /// Requests an allocation of virtual pages, with lower guard pages.
    ///
    /// Returns [`None`] if allocation fails or attemps to create a zero-page allocation.
    ///
    /// Guard pages are included in the total size of the reserved virtual region, but do not count towards
    /// the allocation region itself. This is useful for ensuring that an allocation has unmapped pages below
    /// its region, which is useful for creating a single-allocation stack, for example.
    fn alloc_guarded(&self, page_count: usize, guard_page_count: usize) -> Option<T>;

    /// Releases an existing physical frame allocation.
    fn dealloc(&self, handle: &mut T);
}
