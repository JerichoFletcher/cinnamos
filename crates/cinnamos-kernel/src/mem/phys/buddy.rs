use alloc::collections::linked_list::LinkedList;
use structs::buddy::{BlockIndex, BuddyAllocator};

use crate::{
    arch::{PAddr, PTEFlags, PageSize},
    mem::{self, PAGE_SIZE, SizedMemoryRegion, vms::phys_to_virt},
};

#[derive(Debug)]
pub struct BuddyFrameAlloc {
    id: usize,
    base: PAddr,
    order: usize,
}

impl super::PhysFrameAlloc for BuddyFrameAlloc {
    fn base_addr(&self) -> PAddr {
        self.base
    }
}

#[derive(Debug)]
struct BuddyRegion<'a> {
    id: usize,
    base: PAddr,
    buddy: BuddyAllocator<'a>,
}

impl<'a> BuddyRegion<'a> {
    /// # Safety
    /// - `base` must be aligned to `order` orders of page boundary.
    /// - `next` must point to an aligned buffer of [BlockIndex](BlockIndex) with at least `2 << order` items of capacity.
    /// - `bitmap` must point to an aligned buffer of [u64](u64) with at least `(1 << order).max(64) / 64` items of capacity.
    unsafe fn new(id: usize, base: PAddr, order: usize, next: *mut BlockIndex, bitmap: *mut u64) -> Self {
        assert!(
            Self::max_align_order_of(base) as usize >= order,
            "Base address not aligned: {:016x}, order {}",
            base.addr(),
            order
        );
        let buddy = unsafe { BuddyAllocator::new(order, next, bitmap) };
        Self { id, base, buddy }
    }

    /// `start` and `end` must be within the allocator's memory range
    fn add_range(&mut self, start: PAddr, end: PAddr) {
        assert!(
            self.base <= start && start < self.base + (PAGE_SIZE << self.buddy.max_order()),
            "Range start not within bounds: !(0x{:016x} <= 0x{:016x} < 0x{:016x})",
            self.base,
            start,
            self.base + (PAGE_SIZE << self.buddy.max_order()),
        );
        assert!(
            self.base <= end && end <= self.base + (PAGE_SIZE << self.buddy.max_order()),
            "Range end not within bounds: !(0x{:016x} <= 0x{:016x} < 0x{:016x})",
            self.base,
            end,
            self.base + (PAGE_SIZE << self.buddy.max_order()),
        );
        assert!(
            start <= end,
            "Invalid range: 0x{:016x} .. 0x{:016x}",
            start,
            end
        );

        let count = ((end - start) / PAGE_SIZE) as BlockIndex;
        let start = ((start - self.base) / PAGE_SIZE) as BlockIndex;
        self.buddy.add_blocks(start, count);
    }

    fn alloc(&mut self, frame_count: usize) -> Option<BuddyFrameAlloc> {
        let order = (BlockIndex::BITS - 1 - (frame_count as BlockIndex).leading_zeros()) as usize;
        let block = self.buddy.alloc(order)?;
        let base = self.base + block as usize * PAGE_SIZE;
        Some(BuddyFrameAlloc { id: self.id, base, order })
    }
    
    fn dealloc(&mut self, handle: &mut BuddyFrameAlloc) {
        let block = (handle.base - self.base) / PAGE_SIZE;
        self.buddy.dealloc(handle.order, block as BlockIndex);
    }

    const fn free_count(&self) -> usize {
        self.buddy.free_count()
    }

    pub const fn max_align_order_of(pa: PAddr) -> usize {
        pa.ppn().trailing_zeros() as usize
    }

    pub const fn order_of_size(size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        (BlockIndex::BITS - 1 - ((size / PAGE_SIZE) as BlockIndex).leading_zeros()) as usize
    }
}

#[derive(Debug)]
pub struct BuddyFrameAllocator<'a> {
    regions: LinkedList<BuddyRegion<'a>>,
}

impl<'a> BuddyFrameAllocator<'a> {
    pub fn new(init_regions: &[SizedMemoryRegion]) -> Self {
        let mut instance = Self {
            regions: LinkedList::new(),
        };
        for reg in init_regions {
            instance.add_region(reg);
        }
        instance
    }

    pub fn add_region(&mut self, reg: &SizedMemoryRegion) {
        let size_order = BuddyRegion::order_of_size(reg.size);
        let align_order = BuddyRegion::max_align_order_of(reg.base);

        if size_order != align_order {
            let r_base = PAddr::new(reg.base.addr().next_multiple_of(PAGE_SIZE << size_order));

            let mut l_base = r_base;
            let mut l_order = size_order;
            while l_base > reg.base {
                let diff = l_base - reg.base;
                let ord = BuddyRegion::order_of_size(diff) + 1;
                l_base = l_base - (PAGE_SIZE << ord);
                l_order = ord;
            }
            let r_size = reg.end() - r_base;
            let r_order = BuddyRegion::order_of_size(r_size);

            let mut buf_ptr = reg.base;
            let l_bitmap_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::bitmap_buf_size(l_order) * size_of::<u64>();
            let r_bitmap_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::bitmap_buf_size(r_order) * size_of::<u64>();
            let l_next_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::next_buf_size(l_order) * size_of::<BlockIndex>();
            let r_next_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::next_buf_size(r_order) * size_of::<BlockIndex>();

            mem::vms::acquire(|mut g| {
                let mut pa = l_bitmap_ptr;
                let pa_end = buf_ptr;

                while pa < pa_end {
                    let va = phys_to_virt(pa);
                    let size = PageSize::select_size(va, pa, pa_end - pa).unwrap();
                    let _ = g.map_page(va, pa, size, PTEFlags::RW).map(|o| o.forget());
                    pa = pa + size.size();
                }
            });
            let l_start = buf_ptr.next_multiple_of(PAGE_SIZE);

            if l_start < r_base {
                let mut l_alloc = unsafe {
                    BuddyRegion::new(
                        self.regions.len(),
                        l_base,
                        l_order,
                        phys_to_virt(l_next_ptr).as_mut(),
                        phys_to_virt(l_bitmap_ptr).as_mut(),
                    )
                };

                l_alloc.add_range(l_start, r_base);
                self.regions.push_back(l_alloc);
                crate::println!(
                    "palloc : ORD={} BASE={:?} RANGE={:?} .. {:?}",
                    l_order,
                    l_base,
                    l_start,
                    r_base
                );
            }

            let mut r_alloc = unsafe {
                BuddyRegion::new(
                    self.regions.len(),
                    r_base,
                    r_order,
                    phys_to_virt(r_next_ptr).as_mut(),
                    phys_to_virt(r_bitmap_ptr).as_mut(),
                )
            };

            let r_start = Ord::max(l_start, r_base);
            r_alloc.add_range(r_start, reg.end());
            self.regions.push_back(r_alloc);
            crate::println!(
                "palloc : ORD={} BASE={:?} RANGE={:?} .. {:?}",
                r_order,
                r_base,
                r_start,
                reg.end()
            );
        } else {
            let mut buf_ptr = reg.base;
            let bitmap_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::bitmap_buf_size(size_order) * size_of::<u64>();
            let next_ptr = buf_ptr;
            buf_ptr = buf_ptr + BuddyAllocator::next_buf_size(size_order) * size_of::<BlockIndex>();

            mem::vms::acquire(|mut g| {
                let mut pa = bitmap_ptr;
                let pa_end = buf_ptr;

                while pa < pa_end {
                    let va = phys_to_virt(pa);
                    let size = PageSize::select_size(va, pa, pa_end - pa).unwrap();
                    let _ = g.map_page(va, pa, size, PTEFlags::RW).map(|o| o.forget());
                    pa = pa + size.size();
                }
            });

            let mut alloc = unsafe {
                BuddyRegion::new(
                    self.regions.len(),
                    reg.base,
                    size_order,
                    phys_to_virt(next_ptr).as_mut(),
                    phys_to_virt(bitmap_ptr).as_mut(),
                )
            };
            alloc.add_range(reg.base, reg.end());
            self.regions.push_back(alloc);
            crate::println!(
                "palloc : ORD={} BASE={:?} RANGE={:?} .. {:?}",
                size_order,
                reg.base,
                reg.base,
                reg.end()
            );
        }
    }
}

impl super::PhysFrameAllocator<BuddyFrameAlloc> for BuddyFrameAllocator<'_> {
    fn alloc(&mut self, frame_count: usize) -> Option<BuddyFrameAlloc> {
        for reg in self.regions.iter_mut() {
            if reg.free_count() >= frame_count {
                return reg.alloc(frame_count);
            }
        }
        None
    }

    fn dealloc(&mut self, handle: &mut BuddyFrameAlloc) {
        for reg in self.regions.iter_mut() {
            if reg.id == handle.id {
                reg.dealloc(handle);
                break;
            }
        }
    }
}
