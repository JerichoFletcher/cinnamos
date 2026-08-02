use core::fmt::Debug;

use alloc::collections::BTreeMap;

pub const MAX_ORDER: usize = 36;

cfg_select! {
    target_pointer_width = "64" => {
        pub type BlockIndex = u64;
    }
    target_pointer_width = "32" => {
        pub type BlockIndex = u32;
    }
}

pub const fn order_of(size: BlockIndex) -> usize {
    if size != 0 {
        // Equal to floor(log2(size))
        (BlockIndex::BITS - 1 - size.leading_zeros()) as usize
    } else {
        0
    }
}

pub const fn next_buf_size(order: usize) -> usize {
    2 << order
}

pub fn bitmap_buf_size(order: usize) -> usize {
    (1 << order).max(64) / 64
}

pub trait BackingBuffer {
    fn get_next(&self, index: &BlockIndex) -> BlockIndex;
    fn get_bitmap(&self, index: &BlockIndex) -> u64;

    fn set_next(&mut self, index: &BlockIndex, value: BlockIndex);
    fn set_bitmap(&mut self, index: &BlockIndex, value: u64);
}

pub struct FlatArray {
    next: *mut [BlockIndex],
    bitmap: *mut [u64],
}
impl BackingBuffer for FlatArray {
    fn get_next(&self, index: &BlockIndex) -> BlockIndex {
        // Safety: self.next is valid and exclusive to the backing buffer
        unsafe { (*self.next)[*index as usize] }
    }

    fn get_bitmap(&self, index: &BlockIndex) -> u64 {
        // Safety: self.bitmap is valid and exclusive to the backing buffer
        unsafe { (*self.bitmap)[*index as usize] }
    }

    fn set_next(&mut self, index: &BlockIndex, value: BlockIndex) {
        // Safety: self.next is valid and exclusive to the backing buffer
        unsafe {
            (*self.next)[*index as usize] = value;
        }
    }

    fn set_bitmap(&mut self, index: &BlockIndex, value: u64) {
        // Safety: self.bitmap is valid and exclusive to the backing buffer
        unsafe {
            (*self.bitmap)[*index as usize] = value;
        }
    }
}

pub struct AllocMap {
    next: BTreeMap<BlockIndex, BlockIndex>,
    bitmap: BTreeMap<BlockIndex, u64>,
}
impl BackingBuffer for AllocMap {
    fn get_next(&self, index: &BlockIndex) -> BlockIndex {
        self.next.get(index).copied().unwrap_or(BlockIndex::MAX)
    }

    fn get_bitmap(&self, index: &BlockIndex) -> u64 {
        self.bitmap.get(index).copied().unwrap_or(0)
    }

    fn set_next(&mut self, index: &BlockIndex, value: BlockIndex) {
        if value == BlockIndex::MAX {
            self.next.remove(index);
        } else {
            self.next.insert(*index, value);
        }
    }

    fn set_bitmap(&mut self, index: &BlockIndex, value: u64) {
        if value == 0 {
            self.bitmap.remove(index);
        } else {
            self.bitmap.insert(*index, value);
        }
    }
}

pub struct BuddyAllocator<B: BackingBuffer> {
    order: usize,
    free_lists: [BlockIndex; MAX_ORDER],
    buffers: B,
    total: usize,
    free: usize,
}

impl BuddyAllocator<FlatArray> {
    /// # Safety
    /// - `next` must point to an aligned buffer of [BlockIndex](BlockIndex) with at least [next_buf_size(order)](Self::next_buf_size) items of capacity.
    /// - `bitmap` must point to an aligned buffer of [u64](u64) with at least [bitmap_buf_size(order)](Self::bitmap_buf_size) items of capacity.
    pub unsafe fn new(order: usize, next: *mut [BlockIndex], bitmap: *mut [u64]) -> Self {
        let next_size = next_buf_size(order);
        let bitmap_size = bitmap_buf_size(order);
        assert!(order < MAX_ORDER, "Invalid order: {}", order);
        assert!(
            next.len() >= next_size,
            "next buffer too small ({} vs. {})",
            next.len(),
            next_size
        );
        assert!(
            bitmap.len() >= bitmap_size,
            "bitmap buffer too small ({} vs. {})",
            bitmap.len(),
            bitmap_size
        );

        // Safety: next is valid and is only accessed from this allocator
        unsafe {
            (*next).fill(BlockIndex::MAX);
        }
        // Safety: bitmap is valid and is only accessed from this allocator
        unsafe {
            (*bitmap).fill(0);
        }

        Self {
            order,
            free_lists: [BlockIndex::MAX; MAX_ORDER],
            buffers: FlatArray { next, bitmap },
            total: 0,
            free: 0,
        }
    }
}

impl BuddyAllocator<AllocMap> {
    pub fn new(order: usize) -> Self {
        Self {
            order,
            free_lists: [BlockIndex::MAX; MAX_ORDER],
            buffers: AllocMap {
                next: BTreeMap::new(),
                bitmap: BTreeMap::new(),
            },
            total: 0,
            free: 0,
        }
    }
}

impl<B: BackingBuffer> BuddyAllocator<B> {
    pub fn alloc(&mut self, order: usize) -> Option<BlockIndex> {
        assert!(order < MAX_ORDER, "Invalid order: {}", order);

        // Find smallest free_order >= order
        let free_order = (order..=self.order).find(|&o| self.free_lists[o] != BlockIndex::MAX)?;

        // Pop block from free list
        let block = self.free_lists[free_order];
        let idx = self.next_idx(free_order, block);
        self.free_lists[free_order] = self.buffers.get_next(&idx);
        self.buffers.set_next(&idx, BlockIndex::MAX);
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
            let size_order = order_of(remaining);
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
        self.buffers.set_next(&idx, self.free_lists[order]);
        self.free_lists[order] = block;
    }

    fn free_list_remove(&mut self, order: usize, block: BlockIndex) {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let mut prev = None;
        let mut curr = self.free_lists[order];

        while curr != BlockIndex::MAX {
            let curr_idx = self.next_idx(order, curr);
            if curr == block {
                if let Some(p) = prev {
                    self.buffers.set_next(&p, self.buffers.get_next(&curr_idx));
                } else {
                    self.free_lists[order] = self.buffers.get_next(&curr_idx);
                }
                self.buffers.set_next(&curr_idx, BlockIndex::MAX);
                return;
            }
            prev = Some(curr_idx);
            curr = self.buffers.get_next(&curr_idx);
        }
        panic!("Block {} not found at order {}", block, order);
    }

    const fn order_offset(&self, order: usize) -> BlockIndex {
        (2 << self.order) - (2 << (self.order - order))
    }

    const fn next_idx(&self, order: usize, block: BlockIndex) -> BlockIndex {
        self.order_offset(order) + (block >> (order + 1))
    }

    fn bitmap_bit_get(&self, order: usize, block: BlockIndex) -> bool {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let flat = self.order_offset(order) / 2 + (block >> (order + 1));
        let idx = flat / 64;
        let bit = flat % 64;

        let bits = self.buffers.get_bitmap(&idx);
        (bits >> bit) & 1 == 1
    }

    fn bitmap_bit_toggle(&mut self, order: usize, block: BlockIndex) {
        debug_assert_eq!(block % (1 << order), 0);
        debug_assert!(block <= self.max_block_count());

        let flat = self.order_offset(order) / 2 + (block >> (order + 1));
        let idx = flat / 64;
        let bit = flat % 64;

        let bits = self.buffers.get_bitmap(&idx);
        self.buffers.set_bitmap(&idx, bits ^ (1 << bit));
    }

    fn buddy_of(order: usize, block: BlockIndex) -> BlockIndex {
        debug_assert_eq!(block % (1 << order), 0);
        block ^ (1 << order)
    }
}

impl<B: BackingBuffer> Debug for BuddyAllocator<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BuddyAllocator")
            .field("order", &self.order)
            .field("total", &self.total)
            .field("free", &self.free)
            .field("free_lists", &&self.free_lists[..self.order + 1])
            .finish()
    }
}
