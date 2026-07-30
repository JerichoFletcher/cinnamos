use core::alloc::{GlobalAlloc, Layout};

use spin::RwLock;

mod freelist;

use super::bump;
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
    Bump(&'static dyn Fn(PAddr) -> VAddr),
    Heap,
}
unsafe impl Send for SendHeap {}
unsafe impl Sync for SendHeap {}

static HEAP_MUX: RwLock<SendHeap> = RwLock::new(SendHeap::Bump(&VAddr::identity));
static FREELIST_HEAP: FreeListHeap = FreeListHeap::new(VAddr::new(HEAP_MAP_BASE));

struct HeapImpl;

unsafe impl GlobalAlloc for HeapImpl {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match *HEAP_MUX.read() {
            SendHeap::Bump(p2v) => unsafe { bump::alloc(layout, p2v) },
            SendHeap::Heap => FREELIST_HEAP.alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match *HEAP_MUX.read() {
            SendHeap::Bump(_) => (),
            SendHeap::Heap => FREELIST_HEAP.dealloc(ptr, layout),
        }
    }
}

#[global_allocator]
static ALLOCATOR: HeapImpl = HeapImpl;

/// Should only be called once upon entering higher-half
pub fn shift_bump(p2v: &'static impl Fn(PAddr) -> VAddr) {
    let mut g = HEAP_MUX.write();
    if let SendHeap::Bump(_) = *g {
        *g = SendHeap::Bump(p2v);
    }
}

/// Should only be called once in higher-half
pub fn init_heap() {
    *HEAP_MUX.write() = SendHeap::Heap;
}
