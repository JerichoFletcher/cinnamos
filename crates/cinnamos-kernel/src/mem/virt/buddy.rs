use core::num::NonZero;

use spin::Mutex;
use structs::buddy::{AllocMap, BlockIndex, BuddyAllocator, order_of};

use crate::{arch::VAddr, mem::PAGE_SIZE};

#[derive(Debug)]
pub struct BuddyPageAlloc {
    order: usize,
    block: BlockIndex,
    base: VAddr,
    page_count: NonZero<usize>,
}

impl super::VirtAlloc for BuddyPageAlloc {
    fn start_addr(&self) -> VAddr {
        self.base
    }

    fn end_addr(&self) -> VAddr {
        self.base + self.page_count.get() * PAGE_SIZE
    }

    fn page_count(&self) -> usize {
        self.page_count.get()
    }
}

#[derive(Debug)]
pub struct BuddyVirtAllocator {
    base: VAddr,
    buddy: Mutex<BuddyAllocator<AllocMap>>,
}

impl BuddyVirtAllocator {
    /// # Panic
    /// This function will panic if `start` > `end`, or `start` is not at least aligned to the order of the region size.
    pub fn new(start: VAddr, end: VAddr) -> Self {
        debug_assert!(start <= end);
        let space_size = end - start;
        let size_order = Self::order_of_size(space_size.next_power_of_two());
        let align_order = Self::max_align_order_of(start);
        debug_assert!(size_order <= align_order, "Start is not aligned to size");

        let mut buddy = <BuddyAllocator<AllocMap>>::new(size_order);
        buddy.add_blocks(0, (space_size / PAGE_SIZE) as BlockIndex);
        Self {
            base: start,
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

        let base = self.base + block as usize * PAGE_SIZE;
        Some(BuddyPageAlloc {
            order,
            block,
            base,
            page_count,
        })
    }

    fn alloc_guarded(&self, page_count: usize, guard_page_count: usize) -> Option<BuddyPageAlloc> {
        let mut alloc = self.alloc(page_count + guard_page_count)?;
        alloc.page_count = NonZero::new(page_count)?;
        alloc.base = alloc.base + guard_page_count * PAGE_SIZE;
        Some(alloc)
    }

    fn dealloc(&self, handle: &mut BuddyPageAlloc) {
        self.buddy.lock().dealloc(handle.order, handle.block);
    }
}
