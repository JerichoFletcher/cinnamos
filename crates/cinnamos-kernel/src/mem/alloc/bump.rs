use core::{
    alloc::Layout,
    sync::atomic::{AtomicUsize, Ordering},
};

use spin::Once;

use crate::{
    arch::{PAddr, VAddr},
    mem::PAGE_SIZE,
    sym::{bump_heap_end_p, bump_heap_start_p},
};

#[derive(Debug)]
pub struct BumpAllocator {
    start: PAddr,
    end: PAddr,
    next: AtomicUsize,
}

impl BumpAllocator {
    /// # Safety
    /// `start` and `end` must encompass a valid, read-writable memory space. `start` must align to a page boundary (4 KiB).
    pub unsafe fn new(start: PAddr, end: PAddr) -> Self {
        debug_assert!(start < end);
        Self {
            start,
            end,
            next: AtomicUsize::new(start.addr()),
        }
    }

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

    pub fn alloc_frame(&self, count: usize) -> Option<PAddr> {
        if count == 0 {
            None
        } else {
            // Safety: Passed layout is sized and aligned to PAGE_SIZE
            unsafe { self.alloc(Layout::from_size_align(count * PAGE_SIZE, PAGE_SIZE).ok()?) }
        }
    }

    /// # Safety
    /// `layout` must be a non-zero-sized layout.
    unsafe fn alloc(&self, layout: Layout) -> Option<PAddr> {
        loop {
            let head = self.next.load(Ordering::Relaxed);
            let alloc = head.next_multiple_of(layout.align());
            let next = alloc.checked_add(layout.size())?;
            if next > self.end.addr() {
                return None;
            }

            match self
                .next
                .compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Relaxed)
            {
                Ok(_) => return Some(PAddr::new(alloc)),
                Err(_) => continue,
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

/// # Safety
/// - `layout` must be a non-zero-sized layout.
/// - `p2v` must be a valid physical-to-virtual address translation function within the active virtual address map.
/// - `p2v` must also not change the alignment of physical addresses after translating into virtual addresses.
pub unsafe fn alloc(layout: Layout, p2v: impl Fn(PAddr) -> VAddr) -> *mut u8 {
    // Safety: All safety rules are fulfilled by caller
    unsafe { get_bump().alloc_virt(layout, p2v) }
}

pub fn alloc_frame(count: usize) -> Option<PAddr> {
    get_bump().alloc_frame(count)
}
