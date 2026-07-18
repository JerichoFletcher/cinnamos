use core::alloc::Layout;

use spin::Mutex;

use crate::{
    arch::{PAddr, VAddr},
    bump_heap_end_p, bump_heap_start_p,
    mem::PAGE_SIZE,
};

#[derive(Debug)]
pub struct BumpAllocator {
    curr_addr: PAddr,
    end_addr: PAddr,
}

impl BumpAllocator {
    /// # Safety
    /// `start` and `end` must encompass a valid, read-writable memory space. `start` must align to a page boundary (4 KiB).
    pub unsafe fn new(start: PAddr, end: PAddr) -> Self {
        debug_assert!(start < end);
        Self {
            curr_addr: start,
            end_addr: end,
        }
    }

    /// # Safety
    /// - `layout` must be a non-zero-sized layout.
    /// - `p2v` must be a valid physical-to-virtual address translation function within the active virtual address map.
    /// - `p2v` must also not change the alignment of physical addresses after translating into virtual addresses.
    pub unsafe fn alloc_virt(&mut self, layout: Layout, p2v: impl Fn(PAddr) -> VAddr) -> *mut u8 {
        // Safety: Passed layout is non-zero-sized
        match unsafe { self.alloc(layout) } {
            Some(pa) => p2v(pa).as_mut(),
            None => core::ptr::null_mut(),
        }
    }

    pub fn alloc_frame(&mut self, count: usize) -> Option<PAddr> {
        if count == 0 {
            None
        } else {
            // Safety: Passed layout is sized and aligned to PAGE_SIZE
            unsafe { self.alloc(Layout::from_size_align(count * PAGE_SIZE, PAGE_SIZE).ok()?) }
        }
    }

    /// # Safety
    /// `layout` must be a non-zero-sized layout.
    unsafe fn alloc(&mut self, layout: Layout) -> Option<PAddr> {
        let head = self.curr_addr.addr();
        let alloc = if head % layout.align() == 0 {
            head
        } else {
            head.next_multiple_of(layout.align())
        };
        let next = alloc + layout.size();
        if next > self.end_addr.addr() {
            return None;
        }

        self.curr_addr = PAddr::new(next);
        Some(PAddr::new(alloc))
    }
}

static BUMP_ALLOC: Mutex<Option<BumpAllocator>> = Mutex::new(None);

pub fn init() {
    *BUMP_ALLOC.lock() = unsafe {
        Some(BumpAllocator {
            curr_addr: bump_heap_start_p!(),
            end_addr: bump_heap_end_p!(),
        })
    };
}

/// # Safety
/// - `layout` must be a non-zero-sized layout.
/// - `p2v` must be a valid physical-to-virtual address translation function within the active virtual address map.
/// - `p2v` must also not change the alignment of physical addresses after translating into virtual addresses.
pub unsafe fn alloc(layout: Layout, p2v: impl Fn(PAddr) -> VAddr) -> *mut u8 {
    let mut g = BUMP_ALLOC.lock();
    if let Some(bump) = g.as_mut() {
        unsafe { bump.alloc_virt(layout, p2v) }
    } else {
        core::ptr::null_mut()
    }
}

pub fn alloc_frame(count: usize) -> Option<PAddr> {
    let mut g = BUMP_ALLOC.lock();
    let bump = g.as_mut()?;
    bump.alloc_frame(count)
}
