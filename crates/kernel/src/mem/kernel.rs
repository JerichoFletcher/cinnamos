use core::ptr::{null_mut, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering};
use crate::arch::mem::PT;
use crate::page::zalloc;
use crate::println;

static KMEM_PAGE_TABLE: AtomicPtr<PT> = AtomicPtr::new(null_mut());

pub fn init() {
    match zalloc(1) {
        Some(ptr) => {
            let kmem_pt = ptr.as_ptr().cast::<PT>();
            KMEM_PAGE_TABLE.store(kmem_pt, Ordering::Relaxed);
            println!("Root Page Table       : {:p}", kmem_pt);
        }
        None => panic!("Failed to allocate page table"),
    }
}

#[inline(always)]
pub fn page_table() -> NonNull<PT> {
    NonNull::new(KMEM_PAGE_TABLE.load(Ordering::Relaxed)).unwrap()
}
