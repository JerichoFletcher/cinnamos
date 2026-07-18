use core::alloc::{GlobalAlloc, Layout};

use spin::Mutex;

mod linked;
// mod freelist;

use super::bump;
use crate::{
    arch::{PAddr, VAddr},
    *,
};
use linked::LinkedListHeap as HeapAllocator;

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
    Heap(HeapAllocator),
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
                SendHeap::Heap(heap) => unsafe { heap.alloc(layout) },
            },
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut guard = HEAP.lock();
        if let Some(h) = guard.as_mut() {
            match h {
                SendHeap::Bump(_) => (),
                SendHeap::Heap(heap) => unsafe {
                    heap.dealloc(ptr, layout);
                },
            }
        }
    }
}

#[global_allocator]
static ALLOCATOR: HeapImpl = HeapImpl;

pub fn init_bump() {
    // let heap = HeapAllocator::new(VAddr::new(HEAP_MAP_BASE), HEAP_BUMP_SIZE)?;
    *HEAP.lock() = Some(SendHeap::Bump(&mem::vms::phys_identity));
}

pub fn shift_bump(p2v: &'static impl Fn(PAddr) -> VAddr) {
    let mut g = HEAP.lock();
    if let Some(heap) = g.as_mut()
        && let SendHeap::Bump(_) = heap
    {
        // Safety: This shifts the bump allocator from identity space to higher-half space, which has been mapped
        *g = Some(SendHeap::Bump(p2v));
    }
}

pub fn init_heap() -> Result<(), HeapError> {
    todo!()
}
