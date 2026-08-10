pub mod buddy;

use core::num::NonZero;

use crate::arch::addr::PAddr;

/// A trait for physical frame allocations.
pub trait PhysFrameAlloc {
    /// The base address of the allocated physical region.
    fn start_addr(&self) -> PAddr;

    /// The end address of the allocated physical region.
    fn end_addr(&self) -> PAddr;

    /// The number of physical frames reserved for this allocation.
    fn frame_count(&self) -> NonZero<usize>;

    /// The size of the allocated region, in bytes.
    fn size(&self) -> usize;
}

/// A trait for a physical frame allocator, capable of handing out reservations for physical frames.
pub trait PhysFrameAllocator<T: PhysFrameAlloc> {
    /// Requests an allocation of physical frames.
    /// Returns [`None`] if allocation fails or attempts to create a zero-frame allocation.
    fn alloc(&self, frame_count: usize) -> Option<T>;

    /// Releases an existing physical frame allocation.
    fn dealloc(&self, handle: &mut T);
}
