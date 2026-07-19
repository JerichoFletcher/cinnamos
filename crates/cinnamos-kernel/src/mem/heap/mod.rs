use core::alloc::{GlobalAlloc, Layout};

use spin::Mutex;

mod freelist;

use super::bump;
use crate::{
    arch::{HEAP_MAP_BASE, PAddr, VAddr},
    mem::heap::freelist::FreeListHeap,
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    AllocationFailed,
    MappingFailed,
}

pub trait Heap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}

enum SendHeap {
    Bump(&'static dyn Fn(PAddr) -> VAddr),
    Heap(FreeListHeap),
}

unsafe impl Send for SendHeap {}

static HEAP: Mutex<Option<SendHeap>> = Mutex::new(None);

struct HeapImpl;

unsafe impl GlobalAlloc for HeapImpl {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut guard = HEAP.lock();
        match guard.as_mut() {
            Some(h) => match h {
                SendHeap::Bump(p2v) => unsafe { bump::alloc(layout, *p2v) },
                SendHeap::Heap(heap) => heap.alloc(layout),
            },
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut guard = HEAP.lock();
        if let Some(h) = guard.as_mut() {
            match h {
                SendHeap::Bump(_) => (),
                SendHeap::Heap(heap) => heap.dealloc(ptr, layout),
            }
        }
    }
}

#[global_allocator]
static ALLOCATOR: HeapImpl = HeapImpl;

/// Should only be called once in early phase
pub fn init_bump() {
    *HEAP.lock() = Some(SendHeap::Bump(&mem::vms::phys_identity));
}

/// Should only be called once upon entering higher-half
pub fn shift_bump(p2v: &'static impl Fn(PAddr) -> VAddr) {
    let mut g = HEAP.lock();
    if let Some(wrapper) = g.as_mut()
        && let SendHeap::Bump(_) = wrapper
    {
        *g = Some(SendHeap::Bump(p2v));
    }
}

/// Should only be called once in higher-half
pub fn init_heap() {
    *HEAP.lock() = Some(SendHeap::Heap(FreeListHeap::new(VAddr::new(HEAP_MAP_BASE))));
}
