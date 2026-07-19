use alloc::collections::linked_list::LinkedList;
use structs::buddy::{BlockIndex, BuddyAllocator};

use crate::{arch::VAddr, mem::PAGE_SIZE, *};

pub struct VirtAlloc {
    base: VAddr,
    order: usize,
}

pub struct VirtualRegion {
    base: VAddr,
    end: VAddr,
    buddy: BuddyAllocator,
}

impl VirtualRegion {
    /// `base` must be aligned to `order` orders of page boundary
    pub fn new(base: VAddr, end: VAddr, order: usize) -> Self {
        assert!(
            VirtualAllocator::max_align_order_of(base) as usize >= order,
            "Base address not aligned: {:016x}, order {}",
            base.addr(),
            order
        );
        let mut buddy = BuddyAllocator::new(order);
        let count = (end - base) / PAGE_SIZE;
        buddy.add_blocks(0, count as BlockIndex);

        Self { base, end, buddy }
    }

    pub fn alloc(&mut self, page_count: usize) -> Option<VirtAlloc> {
        let order = (BlockIndex::BITS - 1 - page_count.leading_zeros()) as usize;
        let block = self.buddy.alloc(order)?;
        let base = self.base + block as usize * PAGE_SIZE;
        Some(VirtAlloc { base, order })
    }

    pub fn owns(&self, va: VAddr) -> bool {
        self.base <= va && va < self.base + (PAGE_SIZE << self.buddy.max_order())
    }
}

pub struct VirtualAllocator {
    regions: LinkedList<VirtualRegion>,
}

impl VirtualAllocator {
    pub fn new() -> Self {
        Self {
            regions: LinkedList::new(),
        }
    }

    pub const fn max_align_order_of(va: VAddr) -> usize {
        va.vpn()[0].trailing_zeros() as usize
    }

    pub const fn order_of_size(size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        (BlockIndex::BITS - 1 - ((size / PAGE_SIZE) as BlockIndex).leading_zeros()) as usize
    }
}
