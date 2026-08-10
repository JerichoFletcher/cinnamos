use core::{
    alloc::Layout,
    num::NonZero,
    sync::atomic::{AtomicUsize, Ordering},
};

use spin::Once;

use crate::{
    arch::{PAddr, VAddr},
    mem::{PAGE_SIZE, PhysFrameAlloc},
    sym::{bump_heap_end_p, bump_heap_start_p},
};

/// A frame allocation from the bump space.
#[derive(Debug)]
pub struct BumpFrameAlloc {
    start: PAddr,
    count: NonZero<usize>,
}

impl PhysFrameAlloc for BumpFrameAlloc {
    fn start_addr(&self) -> PAddr {
        self.start
    }

    fn end_addr(&self) -> PAddr {
        self.start + self.count.get() * PAGE_SIZE
    }

    fn size(&self) -> usize {
        self.count.get() * PAGE_SIZE
    }

    fn frame_count(&self) -> NonZero<usize> {
        self.count
    }
}

/// A simple bump allocator that always grows, up to a maximum boundary.
///
/// The bump allocator cannot allocate the same memory location twice during its lifetime, and any allocations will stay
/// valid for the rest of the kernel's life even after the allocator is retired.
#[derive(Debug)]
pub struct BumpAllocator {
    start: PAddr,
    end: PAddr,
    next: AtomicUsize,
}

impl BumpAllocator {
    /// Creates a new bump allocator for the given physical address range.
    ///
    /// # Safety
    /// - `start` and `end` must encompass a valid, read-writable memory space.
    /// - `start` must align to a page boundary (see [`PAGE_SIZE`]).
    pub unsafe fn new(start: PAddr, end: PAddr) -> Self {
        debug_assert!(start < end);
        Self {
            start,
            end,
            next: AtomicUsize::new(start.addr()),
        }
    }

    /// Allocates a memory region as a heap slot.
    /// If allocation fails, this function returns [`null`](core::ptr::null_mut).
    ///
    /// # Safety
    /// - `layout` must be a non-zero-sized layout.
    /// - `p2v` must be a valid physical-to-virtual address translation function within the active virtual address map.
    /// - `p2v` must also not change the alignment of physical addresses after translating into virtual addresses.
    pub unsafe fn alloc_virt(&self, layout: Layout, p2v: impl Fn(PAddr) -> VAddr) -> *mut u8 {
        // Safety: Passed layout is non-zero-sized
        match unsafe { self.alloc(layout) } {
            Some(pa) => p2v(pa).as_mut(),
            None => core::ptr::null_mut(),
        }
    }

    /// Allocates a memory region as a physical frame.
    /// If allocation fails, this function returns [`None`].
    pub fn alloc_frame(&self, count: usize) -> Option<BumpFrameAlloc> {
        let count = NonZero::new(count)?;
        let layout = Layout::from_size_align(count.get() * PAGE_SIZE, PAGE_SIZE).ok()?;

        // Safety: Passed layout is sized and aligned to PAGE_SIZE
        let start = unsafe { self.alloc(layout)? };
        Some(BumpFrameAlloc { start, count })
    }

    /// Allocates a memory slot that adheres to the given layout,
    /// or [`None`] if such a slot cannot be reserved.
    ///
    /// # Safety
    /// `layout` must be a non-zero-sized layout.
    unsafe fn alloc(&self, layout: Layout) -> Option<PAddr> {
        loop {
            let mut head = self.next.load(Ordering::Acquire);
            let alloc = head.next_multiple_of(layout.align());
            let next = alloc.checked_add(layout.size())?;
            if next > self.end.addr() {
                return None;
            }

            match self
                .next
                .compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Some(PAddr::new(alloc)),
                Err(actual) => head = actual,
            }
        }
    }
}

static BUMP_ALLOC: Once<BumpAllocator> = Once::new();

fn get_bump<'a>() -> &'a BumpAllocator {
    BUMP_ALLOC.call_once(|| unsafe { BumpAllocator::new(bump_heap_start_p(), bump_heap_end_p()) })
}

/// Returns `(start, next, end)` addresses of the bump area.
pub fn get_bump_area() -> (PAddr, PAddr, PAddr) {
    let bump = get_bump();
    (
        bump.start,
        PAddr::new(bump.next.load(Ordering::Relaxed)),
        bump.end,
    )
}

/// Allocates a memory region as a heap slot.
/// If allocation fails, this function returns [`null`](core::ptr::null_mut).
///
/// # Safety
/// - `layout` must be a non-zero-sized layout.
/// - `p2v` must be a valid physical-to-virtual address translation function within the active virtual address map.
/// - `p2v` must also not change the alignment of physical addresses after translating into virtual addresses.
pub unsafe fn alloc(layout: Layout, p2v: impl Fn(PAddr) -> VAddr) -> *mut u8 {
    // Safety: All safety rules are fulfilled by caller
    unsafe { get_bump().alloc_virt(layout, p2v) }
}

/// Allocates a memory region as a physical frame.
/// If allocation fails, this function returns
pub fn alloc_frame(count: usize) -> Option<BumpFrameAlloc> {
    get_bump().alloc_frame(count)
}
