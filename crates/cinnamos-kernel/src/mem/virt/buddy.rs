use core::num::NonZero;

use spin::Mutex;
use structs::buddy::{AllocMap, BlockIndex, BuddyAllocator, order_of};

use crate::{arch::VAddr, mem::PAGE_SIZE};

#[derive(Debug)]
pub struct BuddyPageAlloc {
    base: VAddr,
    order: usize,
    page_count: NonZero<usize>,
    reverse: bool,
}

impl super::VirtAlloc for BuddyPageAlloc {
    fn start_addr(&self) -> VAddr {
        if !self.reverse {
            self.base
        } else {
            self.base + ((1 << self.order) - self.page_count.get()) * PAGE_SIZE
        }
    }

    fn end_addr(&self) -> VAddr {
        if !self.reverse {
            self.base + self.page_count.get() * PAGE_SIZE
        } else {
            self.base + (1 << self.order) * PAGE_SIZE
        }
    }
}

#[derive(Debug)]
pub struct BuddyVirtAllocator {
    base: VAddr,
    reverse: bool,
    buddy: Mutex<BuddyAllocator<AllocMap>>,
}

impl BuddyVirtAllocator {
    /// Passing `start` and `end` where `end` < `start` will create a downwards-growing allocator.
    ///
    /// # Panic
    /// This function will panic if `start` is not at least aligned to the order of the region size.
    pub fn new(start: VAddr, end: VAddr) -> Self {
        let reverse = end < start;
        let space_size = if !reverse { end - start } else { start - end };

        let base = if !reverse { start } else { end };
        let size_order = Self::order_of_size(space_size.next_power_of_two());
        let align_order = Self::max_align_order_of(base);
        debug_assert!(size_order <= align_order, "Start is not aligned to size");

        let mut buddy = <BuddyAllocator<AllocMap>>::new(size_order);
        buddy.add_blocks(0, (space_size / PAGE_SIZE) as BlockIndex);
        Self {
            base: start,
            reverse,
            buddy: Mutex::new(buddy),
        }
    }

    pub const fn order_of_size(size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        order_of((size / PAGE_SIZE) as _)
    }

    pub const fn max_align_order_of(va: VAddr) -> usize {
        va.vpn_all().trailing_zeros() as usize
    }
}

impl super::VirtAllocator<BuddyPageAlloc> for BuddyVirtAllocator {
    fn alloc(&self, page_count: usize) -> Option<BuddyPageAlloc> {
        let page_count = NonZero::new(page_count)?;
        let page_count_fit = page_count.get().next_power_of_two();
        let order = order_of(page_count_fit.try_into().ok()?);
        let block = self.buddy.lock().alloc(order)?;

        let base = if !self.reverse {
            self.base + block as usize * PAGE_SIZE
        } else {
            self.base - (block + (1 << order)) as usize * PAGE_SIZE
        };

        Some(BuddyPageAlloc {
            base,
            order,
            page_count,
            reverse: self.reverse,
        })
    }

    fn dealloc(&self, handle: &mut BuddyPageAlloc) {
        let block = (handle.base - self.base) / PAGE_SIZE;
        self.buddy.lock().dealloc(handle.order, block as BlockIndex);
    }
}
