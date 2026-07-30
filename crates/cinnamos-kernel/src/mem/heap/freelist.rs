use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use spin::Mutex;

use crate::{
    arch::{PTEFlags, VAddr},
    mem::{PhysFrameAlloc, physalloc, vms},
};

const LO_HEAP_FRAMES: usize = 1;
const MD_HEAP_FRAMES: usize = 8;
const HI_HEAP_FRAMES: usize = 128;

const MIN_ALLOC_SIZE: usize = 8;
const SIZES_LO: [usize; 5] = [8, 16, 32, 64, 128];
const SIZES_MD: [usize; 5] = [256, 512, 1024, 2048, 4096];
const SIZES_HI: [usize; 5] = [8192, 16384, 32768, 65536, 131072];

enum BlockSizeLookup {
    Invalid,
    Lo(usize),
    Md(usize),
    Hi(usize),
}

struct FreeBlock {
    next: *mut FreeBlock,
}

pub struct FreeListHeap {
    next_va: AtomicUsize,
    pool_heads_lo: [Mutex<*mut FreeBlock>; SIZES_LO.len()],
    pool_heads_md: [Mutex<*mut FreeBlock>; SIZES_MD.len()],
    pool_heads_hi: [Mutex<*mut FreeBlock>; SIZES_HI.len()],
}

unsafe impl Sync for FreeListHeap {}

impl FreeListHeap {
    pub const fn new(base: VAddr) -> Self {
        Self {
            next_va: AtomicUsize::new(base.addr()),
            pool_heads_lo: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_LO.len()],
            pool_heads_md: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_MD.len()],
            pool_heads_hi: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_HI.len()],
        }
    }

    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        let layout = layout.pad_to_align();
        let block_size = layout.size().next_power_of_two().max(MIN_ALLOC_SIZE);
        self.alloc_block(block_size)
    }

    pub fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let layout = layout.pad_to_align();
        let block_size = layout.size().next_power_of_two().max(MIN_ALLOC_SIZE);
        self.dealloc_block(ptr, block_size);
    }

    fn alloc_block(&self, size: usize) -> *mut u8 {
        let (pool, heap_grow_frames) = match Self::lookup_block_size(size) {
            BlockSizeLookup::Lo(i) => (&self.pool_heads_lo[i], LO_HEAP_FRAMES),
            BlockSizeLookup::Md(i) => (&self.pool_heads_md[i], MD_HEAP_FRAMES),
            BlockSizeLookup::Hi(i) => (&self.pool_heads_hi[i], HI_HEAP_FRAMES),
            BlockSizeLookup::Invalid => return core::ptr::null_mut(),
        };

        let mut head = pool.lock();
        match NonNull::new(*head) {
            Some(block) => {
                let next = unsafe { block.as_ref().next };
                *head = next;
                block.as_ptr().cast()
            }
            None => match physalloc::alloc(heap_grow_frames) {
                Some(alloc) => {
                    drop(head);

                    let alloc_size = alloc.size();
                    let next_va = self.next_va.fetch_add(alloc_size, Ordering::Relaxed);
                    let base_va = VAddr::new(next_va);
                    let end_va = base_va + alloc_size;

                    match vms::map_raw(
                        base_va,
                        alloc.start_addr(),
                        alloc_size,
                        PTEFlags::GLOBAL | PTEFlags::RW,
                    ) {
                        Ok(()) => {
                            let mut next_va = VAddr::NULL;
                            let mut prev_va = end_va - size;
                            let tail_block = prev_va.as_mut::<FreeBlock>();

                            while prev_va >= base_va {
                                // Safety: prev_va is within base_va and end_va, which is mapped
                                unsafe {
                                    *prev_va.as_mut::<FreeBlock>() = FreeBlock {
                                        next: next_va.as_mut(),
                                    }
                                }
                                next_va = prev_va;
                                prev_va = prev_va - size;
                            }
                            let head_block = next_va.as_mut::<FreeBlock>();

                            let mut head = pool.lock();
                            if !(*head).is_null() {
                                // Safety: tail is the last block within the pages we allocated
                                unsafe {
                                    (*tail_block).next = *head;
                                }
                            }
                            // Safety: head_block is the first block within the pages we allocated
                            *head = unsafe { (*head_block).next };
                            head_block.cast()
                        }
                        Err(_) => core::ptr::null_mut(),
                    }
                    // if vms::acquire(|mut g| {
                    //     g.map_pages_and_forget(
                    //         alloc.start_addr(),
                    //         alloc.end_addr(),
                    //         base_va,
                    //         PTEFlags::GLOBAL | PTEFlags::RW,
                    //     )
                    // })
                    // .is_ok()
                    // {
                    //     core::mem::forget(alloc);

                    //     let mut next_va = VAddr::NULL;
                    //     let mut prev_va = end_va - size;
                    //     let tail_block = prev_va.as_mut::<FreeBlock>();

                    //     while prev_va >= base_va {
                    //         // Safety: prev_va is within base_va and end_va, which is mapped
                    //         unsafe {
                    //             *prev_va.as_mut::<FreeBlock>() = FreeBlock {
                    //                 next: next_va.as_mut(),
                    //             }
                    //         }
                    //         next_va = prev_va;
                    //         prev_va = prev_va - size;
                    //     }
                    //     let head_block = next_va.as_mut::<FreeBlock>();

                    //     let mut head = pool.lock();
                    //     if !(*head).is_null() {
                    //         // Safety: tail is the last block within the pages we allocated
                    //         unsafe {
                    //             (*tail_block).next = *head;
                    //         }
                    //     }
                    //     // Safety: head_block is the first block within the pages we allocated
                    //     *head = unsafe { (*head_block).next };
                    //     head_block.cast()
                    // } else {
                    //     core::ptr::null_mut()
                    // }
                }
                None => core::ptr::null_mut(),
            },
        }
    }

    fn dealloc_block(&self, ptr: *mut u8, size: usize) {
        let pool = match Self::lookup_block_size(size) {
            BlockSizeLookup::Lo(i) => &self.pool_heads_lo[i],
            BlockSizeLookup::Md(i) => &self.pool_heads_md[i],
            BlockSizeLookup::Hi(i) => &self.pool_heads_hi[i],
            BlockSizeLookup::Invalid => panic!("Invalid block size {size}"),
        };

        let mut head = pool.lock();
        let new_head: *mut FreeBlock = ptr.cast();
        unsafe {
            (*new_head).next = *head;
        }
        *head = new_head;
    }

    fn lookup_block_size(size: usize) -> BlockSizeLookup {
        if size <= SIZES_LO[4] {
            for (i, s) in SIZES_LO.iter().enumerate() {
                if size == *s {
                    return BlockSizeLookup::Lo(i);
                }
            }
        } else if size <= SIZES_MD[4] {
            for (i, s) in SIZES_MD.iter().enumerate() {
                if size == *s {
                    return BlockSizeLookup::Md(i);
                }
            }
        } else if size <= SIZES_HI[4] {
            for (i, s) in SIZES_HI.iter().enumerate() {
                if size == *s {
                    return BlockSizeLookup::Hi(i);
                }
            }
        }
        BlockSizeLookup::Invalid
    }
}
