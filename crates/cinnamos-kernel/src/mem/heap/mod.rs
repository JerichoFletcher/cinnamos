use core::alloc::{GlobalAlloc, Layout};

use spin::Mutex;

mod linked;
use linked::LinkedListHeap as HeapAllocator;

use crate::{arch::{HEAP_BUMP_SIZE, HEAP_MAP_BASE, VAddr}, println};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    AllocationFailed,
    MappingFailed,
}

pub trait Heap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout);
}

struct SendHeap(HeapAllocator);

unsafe impl Send for SendHeap {}

static HEAP: Mutex<Option<SendHeap>> = Mutex::new(None);

struct HeapImpl;

unsafe impl GlobalAlloc for HeapImpl {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut guard = HEAP.lock();
        match guard.as_mut() {
            Some(h) => unsafe { h.0.alloc(layout) },
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut guard = HEAP.lock();
        if let Some(h) = guard.as_mut() {
            unsafe { h.0.dealloc(ptr, layout); }
        }
    }
}

#[global_allocator]
static ALLOCATOR: HeapImpl = HeapImpl;

pub fn init() -> Result<(), HeapError> {
    let heap = HeapAllocator::new(VAddr::new(HEAP_MAP_BASE), HEAP_BUMP_SIZE)?;
    println!("heap : {:?}", heap);
    *HEAP.lock() = Some(SendHeap(heap));
    Ok(())
}
