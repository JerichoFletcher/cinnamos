use core::{alloc::Layout, ptr::NonNull};

use spin::Mutex;

use crate::{
    arch::{PTEFlags, PageSize, VAddr},
    mem::{PhysFrameAlloc, palloc, vms},
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
    next_va: VAddr,
    pool_heads_lo: [Mutex<*mut FreeBlock>; SIZES_LO.len()],
    pool_heads_md: [Mutex<*mut FreeBlock>; SIZES_MD.len()],
    pool_heads_hi: [Mutex<*mut FreeBlock>; SIZES_HI.len()],
}

impl FreeListHeap {
    pub fn new(base: VAddr) -> Self {
        Self {
            next_va: base,
            pool_heads_lo: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_LO.len()],
            pool_heads_md: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_MD.len()],
            pool_heads_hi: [const { Mutex::new(core::ptr::null_mut()) }; SIZES_HI.len()],
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let layout = layout.pad_to_align();
        let block_size = layout.size().next_power_of_two().max(MIN_ALLOC_SIZE);
        if block_size <= SIZES_LO[SIZES_LO.len() - 1] {
            self.alloc_block(block_size)
        } else {
            core::ptr::null_mut()
        }
    }

    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let layout = layout.pad_to_align();
        let block_size = layout.size().next_power_of_two().max(MIN_ALLOC_SIZE);
        if block_size <= SIZES_LO[SIZES_LO.len() - 1] {
            self.dealloc_block(ptr, block_size);
        }
    }

    fn alloc_block(&mut self, size: usize) -> *mut u8 {
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
            None => match palloc::alloc(heap_grow_frames) {
                Some(alloc) => {
                    let alloc_size = alloc.size();
                    let base_va = self.next_va;
                    let end_va = base_va + alloc_size;
                    self.next_va = end_va;

                    if vms::acquire(|mut g| {
                        let mut va = base_va;
                        let mut pa = alloc.start_addr();
                        let pa_end = alloc.end_addr();
                        while pa < pa_end {
                            let next_size = PageSize::select_size(va, pa, pa_end - pa).ok_or(())?;
                            g.map_page(va, pa, next_size, PTEFlags::RW)
                                .map_err(|_| ())?
                                .forget();
                            pa = pa + next_size.size();
                            va = va + next_size.size();
                        }
                        Ok::<(), ()>(())
                    })
                    .is_ok()
                    {
                        core::mem::forget(alloc);

                        let mut next_va = VAddr::NULL;
                        let mut prev_va = end_va - size;
                        while prev_va >= base_va {
                            unsafe {
                                *prev_va.as_mut::<FreeBlock>() = FreeBlock {
                                    next: next_va.as_mut(),
                                }
                            }
                            next_va = prev_va;
                            prev_va = prev_va - size;
                        }
                        let head_block = next_va.as_mut::<FreeBlock>();
                        *head = unsafe { (*head_block).next };
                        head_block.cast()
                    } else {
                        core::ptr::null_mut()
                    }
                }
                None => core::ptr::null_mut(),
            },
        }
    }

    fn dealloc_block(&mut self, ptr: *mut u8, size: usize) {
        let pool = match size {
            8 => &self.pool_heads_lo[0],
            16 => &self.pool_heads_lo[1],
            32 => &self.pool_heads_lo[2],
            64 => &self.pool_heads_lo[3],
            128 => &self.pool_heads_lo[4],
            _ => panic!("Invalid size {}", size),
        };

        let mut head = pool.lock();
        let new_head: *mut FreeBlock = ptr.cast();
        unsafe {
            (*new_head).next = *head;
        }
        *head = new_head;
    }

    fn lookup_block_size(size: usize) -> BlockSizeLookup {
        for (i, s) in SIZES_LO.iter().enumerate() {
            if size == *s {
                return BlockSizeLookup::Lo(i);
            }
        }
        for (i, s) in SIZES_MD.iter().enumerate() {
            if size == *s {
                return BlockSizeLookup::Md(i);
            }
        }
        for (i, s) in SIZES_HI.iter().enumerate() {
            if size == *s {
                return BlockSizeLookup::Hi(i);
            }
        }
        BlockSizeLookup::Invalid
    }
}
