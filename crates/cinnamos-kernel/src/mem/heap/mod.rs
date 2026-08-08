use core::alloc::{GlobalAlloc, Layout};

use spin::RwLock;

mod freelist;

use super::alloc::bump;
use crate::{
    arch::{HEAP_MAP_BASE, PAddr, VAddr},
    mem::heap::freelist::FreeListHeap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    AllocationFailed,
    MappingFailed,
}

pub trait Heap {
    /// # Safety
    /// `layout` must have non-zero size.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;

    /// # Safety
    /// - `ptr` must point to a block allocated in this heap.
    /// - `layout` must be the same layout to allocate the block.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}

enum SendHeap {
    Bump(fn(PAddr) -> VAddr),
    Heap,
}

static HEAP_MUX: RwLock<SendHeap> = RwLock::new(SendHeap::Bump(VAddr::identity));
static FREELIST_HEAP: FreeListHeap = FreeListHeap::new(VAddr::new(HEAP_MAP_BASE));

struct HeapImpl;

unsafe impl GlobalAlloc for HeapImpl {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match *HEAP_MUX.read() {
            SendHeap::Bump(p2v) => unsafe { bump::alloc(layout, p2v) },
            // Safety: layout is non-zero-sized
            SendHeap::Heap => unsafe { FREELIST_HEAP.alloc(layout) },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match *HEAP_MUX.read() {
            SendHeap::Bump(_) => (),
            // Safety: GlobalAlloc requires that layout be the one used to allocate ptr
            SendHeap::Heap => unsafe { FREELIST_HEAP.dealloc(ptr, layout) },
        }
    }
}

#[global_allocator]
static ALLOCATOR: HeapImpl = HeapImpl;

/// Sets a new address translator for bump allocations.
///
/// Should only be called once upon entering higher-half.
///
/// # Safety
/// `p2v` must map the bump space into a mapped virtual space.
pub unsafe fn shift_bump(p2v: fn(PAddr) -> VAddr) {
    let mut g = HEAP_MUX.write();
    if matches!(*g, SendHeap::Bump(_)) {
        *g = SendHeap::Bump(p2v);
    }
}

/// Retires the bump allocator and switches the [`GlobalAlloc`] backend to the [`FreeListHeap`].
///
/// Should only be called once in higher-half.
pub fn init_heap() {
    log::debug!("enabling freelist heap");
    *HEAP_MUX.write() = SendHeap::Heap;
}
