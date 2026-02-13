use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use bitflags::bitflags;
use crate::arch::mem::{PAGE_SIZE, PAGE_SIZE_ORD};
use crate::println;

unsafe extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}
static MAX_PAGES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_START: AtomicUsize = AtomicUsize::new(0);

bitflags! {
    struct PageFlag : u8 {
        const Empty = 0;
        const Taken = 1 << 0;
        const Last = 1 << 1;
    }
}

struct Page {
    flags: PageFlag,
}

impl Page {
    #[inline]
    fn is_free(&self) -> bool {
        !self.is_taken()
    }

    #[inline]
    fn is_taken(&self) -> bool {
        self.flags.contains(PageFlag::Taken)
    }

    #[inline]
    fn is_last(&self) -> bool {
        self.flags.contains(PageFlag::Last)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.flags = PageFlag::Empty;
    }

    #[inline]
    pub fn set_flag(&mut self, flag: PageFlag) {
        self.flags.insert(flag);
    }
}

pub fn init() {
    unsafe {
        let page_count = HEAP_SIZE / PAGE_SIZE;
        MAX_PAGES.store(page_count, Ordering::Relaxed);

        let ptr = HEAP_START as *mut Page;
        for i in 0..page_count {
            (*ptr.add(i)).clear();
        }
        let ptr_end = ptr.add(page_count - 1);
        let alloc_start = crate::bits::align_next(ptr_end.addr(), PAGE_SIZE_ORD);
        let alloc_end = alloc_start + page_count * PAGE_SIZE - 1;
        ALLOC_START.store(alloc_start, Ordering::Relaxed);

        println!("Heap Meta             : {:p} -> {:p}", ptr, ptr_end);
        println!("Heap Space            : 0x{:x} -> 0x{:x}", alloc_start, alloc_end);
        println!("Page Count            : {}", page_count);
        println!("Page Size             : {} B", PAGE_SIZE);
    }
}

pub fn alloc(n_page: usize) -> Option<NonNull<u8>> {
    assert!(n_page > 0, "Allocating zero pages");
    unsafe {
        let max_pages = MAX_PAGES.load(Ordering::Relaxed);
        let start_ptr = HEAP_START as *mut Page;

        let mut i = 0;
        let mut found = false;

        while i < max_pages - n_page {
            if (*start_ptr.add(i)).is_free() {
                found = true;
                for j in i..i + n_page {
                    if (*start_ptr.add(j)).is_taken() {
                        found = false;
                        i = j + 1;
                        break;
                    }
                }
                if found { break; }
            } else {
                i += 1;
            }
        }

        if found {
            for j in i..i + n_page {
                (*start_ptr.add(j)).set_flag(PageFlag::Taken);
                if j == i + n_page - 1 {
                    (*start_ptr.add(j)).set_flag(PageFlag::Last);
                }
            }
            NonNull::new((ALLOC_START.load(Ordering::Relaxed) + i * PAGE_SIZE) as *mut u8)
        } else {
            None
        }
    }
}

pub fn dealloc(ptr: NonNull<u8>) {
    unsafe {
        let desc_addr = HEAP_START + (ptr.addr().get() - ALLOC_START.load(Ordering::Relaxed)) / PAGE_SIZE;
        assert!(
            desc_addr >= HEAP_START && desc_addr < HEAP_START + HEAP_SIZE,
            "Invalid heap pointer"
        );

        let mut ptr = desc_addr as *mut Page;
        while (*ptr).is_taken() && !(*ptr).is_last() {
            (*ptr).clear();
            ptr = ptr.add(1);
        }

        assert!((*ptr).is_last(), "Free page encountered before last page (possible double-free)");
        (*ptr).clear();
    }
}

pub fn zalloc(page_count: usize) -> Option<NonNull<u8>> {
    if let Some(p8) = alloc(page_count) {
        let size = (PAGE_SIZE * page_count) / 8;
        let p64 = p8.cast::<u64>();
        for i in 0..size {
            unsafe {
                p64.add(i).write(0);
            }
        }
        Some(p8)
    } else {
        None
    }
}
