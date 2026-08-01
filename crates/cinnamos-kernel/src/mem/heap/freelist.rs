use core::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

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
    pool_heads_lo: [AtomicPtr<FreeBlock>; SIZES_LO.len()],
    pool_heads_md: [AtomicPtr<FreeBlock>; SIZES_MD.len()],
    pool_heads_hi: [AtomicPtr<FreeBlock>; SIZES_HI.len()],
}

unsafe impl Sync for FreeListHeap {}

impl FreeListHeap {
    pub const fn new(base: VAddr) -> Self {
        Self {
            next_va: AtomicUsize::new(base.addr()),
            pool_heads_lo: [const { AtomicPtr::new(core::ptr::null_mut()) }; SIZES_LO.len()],
            pool_heads_md: [const { AtomicPtr::new(core::ptr::null_mut()) }; SIZES_MD.len()],
            pool_heads_hi: [const { AtomicPtr::new(core::ptr::null_mut()) }; SIZES_HI.len()],
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

        let mut head = pool.load(Ordering::Acquire);
        loop {
            if !head.is_null() {
                let next = unsafe { (*head).next };
                match pool.compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Acquire) {
                    Ok(_) => return head.cast(),
                    Err(x) => head = x,
                }
            } else {
                break;
            }
        }

        match physalloc::alloc(heap_grow_frames) {
            Some(alloc) => {
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
                        let used_block = next_va.as_mut::<FreeBlock>();
                        // Safety: head_block is the first block within the pages we allocated
                        let head_block = unsafe { (*used_block).next };

                        let mut head = pool.load(Ordering::Acquire);
                        loop {
                            if !head.is_null() {
                                // Safety: tail is the last block within the pages we allocated
                                unsafe {
                                    (*tail_block).next = head;
                                }
                            }
                            match pool.compare_exchange_weak(
                                head,
                                head_block,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            ) {
                                Ok(_) => {
                                    core::mem::forget(alloc);
                                    return used_block.cast();
                                }
                                Err(x) => head = x,
                            }
                        }
                    }
                    Err(_) => core::ptr::null_mut(),
                }
            }
            None => core::ptr::null_mut(),
        }
    }

    fn dealloc_block(&self, ptr: *mut u8, size: usize) {
        let pool = match Self::lookup_block_size(size) {
            BlockSizeLookup::Lo(i) => &self.pool_heads_lo[i],
            BlockSizeLookup::Md(i) => &self.pool_heads_md[i],
            BlockSizeLookup::Hi(i) => &self.pool_heads_hi[i],
            BlockSizeLookup::Invalid => panic!("Invalid block size {size}"),
        };

        loop {
            let head = pool.load(Ordering::Relaxed);
            let new_head: *mut FreeBlock = ptr.cast();
            unsafe {
                (*new_head).next = head;
            }
            match pool.compare_exchange_weak(head, new_head, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
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
