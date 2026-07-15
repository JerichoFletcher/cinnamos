use core::ptr::NonNull;

use spin::Mutex;

use crate::mem::PAGE_SIZE;

const LO_HEAP_SIZE: usize = 2 * PAGE_SIZE;

const SIZES_LO: [usize; 4] = [64, 128, 256, 512];

struct FreeBlock {
    next: *mut FreeBlock,
}

pub struct FreeListHeap {
    pool_heads_lo: [Mutex<*mut FreeBlock>; SIZES_LO.len()],
}

impl FreeListHeap {
    fn lo_alloc(&mut self, size: usize) -> *mut () {
        let pool = match size {
            64 => &self.pool_heads_lo[0],
            128 => &self.pool_heads_lo[1],
            256 => &self.pool_heads_lo[2],
            512 => &self.pool_heads_lo[3],
            _ => return core::ptr::null_mut(),
        };

        let mut head = pool.lock();
        match NonNull::new(*head) {
            Some(block) => {
                let next = unsafe { block.as_ref().next };
                *head = next;
                block.as_ptr().cast()
            },
            None => {
                // TODO: Allocate more pages
                core::ptr::null_mut()
            }
        }
    }
}
