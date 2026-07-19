use core::fmt::Debug;

pub const MAX_ORDER: usize = 32;

pub type BlockIndex = u32;

pub struct BuddyAllocator<'a> {
    order: usize,
    free_lists: [BlockIndex; MAX_ORDER],
    next: &'a mut [BlockIndex],
    bitmap: &'a mut [u64],
    total: usize,
    free: usize,
}

impl<'a> BuddyAllocator<'a> {
    pub const fn next_buf_size(order: usize) -> usize {
        2 << order
    }

    pub fn bitmap_buf_size(order: usize) -> usize {
        (1 << order).max(64) / 64
    }

    /// # Safety
    /// - `next` must point to an aligned buffer of [BlockIndex](BlockIndex) with at least [next_buf_size(order)](Self::next_buf_size) items of capacity.
    /// - `bitmap` must point to an aligned buffer of [u64](u64) with at least [bitmap_buf_size(order)](Self::bitmap_buf_size) items of capacity.
    pub unsafe fn new(order: usize, next: *mut BlockIndex, bitmap: *mut u64) -> Self {
        assert!(order < MAX_ORDER, "Invalid order: {}", order);
        let next_size = Self::next_buf_size(order);
        let bitmap_size = Self::bitmap_buf_size(order);

        let next =
            unsafe { core::ptr::slice_from_raw_parts_mut(next, next_size).as_mut_unchecked() };
        let bitmap =
            unsafe { core::ptr::slice_from_raw_parts_mut(bitmap, bitmap_size).as_mut_unchecked() };

        Self {
            order,
            free_lists: [BlockIndex::MAX; MAX_ORDER],
            next,
            bitmap,
            total: 0,
            free: 0,
        }
    }

    pub fn alloc(&mut self, order: usize) -> Option<BlockIndex> {
        assert!(order < MAX_ORDER, "Invalid order: {}", order);

        // Find smallest free_order >= order
        let free_order = (order..=self.order).find(|&o| self.free_lists[o] != BlockIndex::MAX)?;

        // Pop block from free list
        let block = self.free_lists[free_order];
        let idx = self.next_idx(free_order, block);
        self.free_lists[free_order] = self.next[idx];
        self.next[idx] = BlockIndex::MAX;
        self.bitmap_bit_toggle(free_order, block);

        // Split and push children
        let mut current = block;
        for o in (order..free_order).rev() {
            let child_l = current;
            let child_r = child_l + (1 << o);

            self.free_list_push(o, child_r);
            self.bitmap_bit_toggle(o, child_r);

            current = child_l;
        }

        self.free -= 1 << order;
        Some(current)
    }

    pub fn dealloc(&mut self, order: usize, block: BlockIndex) {
        assert!(order < MAX_ORDER, "Invalid order: {}", order);

        let mut curr_order = order;
        let mut curr_block = block;

        loop {
            self.bitmap_bit_toggle(curr_order, curr_block);

            if curr_order < self.order && !self.bitmap_bit_get(curr_order, curr_block) {
                // Bit is 0: buddy is also deallocated and safe to merge
                let buddy = Self::buddy_of(curr_order, curr_block);
                self.free_list_remove(curr_order, buddy);
                curr_block &= !(1 << curr_order);
                curr_order += 1;
            } else {
                break;
            }
        }
        self.free_list_push(curr_order, curr_block);
        self.free += 1 << order;
    }

    pub fn add_blocks(&mut self, start: BlockIndex, count: BlockIndex) {
        assert!(
            start.checked_add(count).is_some(),
            "Block index overflow: {} + {}",
            start,
            count,
        );
        assert!(
            start + count <= self.max_block_count(),
            "Blocks out of range: {} vs. {}",
            start + count,
            self.max_block_count(),
        );

        let mut idx = start;
        let end = start + count;

        while idx < end {
            let remaining = end - idx;

            // Available order by alignment
            let align_order = if idx == 0 {
                self.order
            } else {
                (idx.trailing_zeros() as usize).min(self.order)
            };

            // Available order by size
            // size_order = floor(log2(remaining))
            let size_order = (BlockIndex::BITS - 1 - remaining.leading_zeros()) as usize;
            let size_order = size_order.min(self.order);

            // Choose smaller available order
            let eff_order = align_order.min(size_order);
            let eff_size = 1 << eff_order;

            self.dealloc(eff_order, idx);
            self.total += eff_size;

            idx += eff_size as BlockIndex;
        }
    }

    pub const fn max_order(&self) -> usize {
        self.order
    }

    pub const fn max_block_count(&self) -> BlockIndex {
        1 << self.order
    }

    pub const fn free_count(&self) -> usize {
        self.free
    }

    fn free_list_push(&mut self, order: usize, block: BlockIndex) {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());
        
        let idx = self.next_idx(order, block);
        self.next[idx] = self.free_lists[order];
        self.free_lists[order] = block;
    }
    
    fn free_list_remove(&mut self, order: usize, block: BlockIndex) {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let mut prev: Option<usize> = None;
        let mut curr = self.free_lists[order];

        while curr != BlockIndex::MAX {
            let curr_idx = self.next_idx(order, curr);
            if curr == block {
                if let Some(p) = prev {
                    self.next[p] = self.next[curr_idx];
                } else {
                    self.free_lists[order] = self.next[curr_idx];
                }
                self.next[curr_idx] = BlockIndex::MAX;
                return;
            }
            prev = Some(curr_idx);
            curr = self.next[curr_idx];
        }
        panic!("Block {} not found at order {}", block, order);
    }

    const fn order_offset(&self, order: usize) -> usize {
        (2 << self.order) - (2 << (self.order - order))
    }

    const fn next_idx(&self, order: usize, block: BlockIndex) -> usize {
        self.order_offset(order) + (block as usize >> (order + 1))
    }

    fn bitmap_bit_get(&self, order: usize, block: BlockIndex) -> bool {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let flat = self.order_offset(order) / 2 + (block as usize >> (order + 1));
        let idx = flat / 64;
        let bit = flat % 64;
        (self.bitmap[idx] >> bit) & 1 == 1
    }

    fn bitmap_bit_toggle(&mut self, order: usize, block: BlockIndex) {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let flat = self.order_offset(order) / 2 + (block as usize >> (order + 1));
        let idx = flat / 64;
        let bit = flat % 64;
        self.bitmap[idx] ^= 1 << bit;
    }

    fn buddy_of(order: usize, block: BlockIndex) -> BlockIndex {
        debug_assert_eq!(block % (1 << order), 0);
        block ^ (1 << order)
    }
}

impl Debug for BuddyAllocator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BuddyAllocator")
            .field("order", &self.order)
            .field("total", &self.total)
            .field("free", &self.free)
            .field("free_lists", &self.free_lists.split_at(self.order + 1).0)
            .finish()
    }
}
