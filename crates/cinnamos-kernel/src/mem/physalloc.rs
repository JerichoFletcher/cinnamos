use core::cmp::Reverse;

use fdt::Fdt;
use spin::RwLock;

use crate::{
    arch::PAddr,
    mem::{
        PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion,
        phys::{
            PhysFrameAllocator,
            buddy::{BuddyFrameAlloc, BuddyFrameAllocator},
        },
    },
    sym::*,
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PAllocError {
    AllocatorUninitialized,
}

#[derive(Debug)]
pub enum Alloc {
    BumpAlloc((PAddr, usize)),
    BuddyAlloc(BuddyFrameAlloc),
}

impl Drop for Alloc {
    fn drop(&mut self) {
        dealloc(self);
    }
}

impl PhysFrameAlloc for Alloc {
    fn start_addr(&self) -> PAddr {
        match self {
            Alloc::BumpAlloc((pa, _)) => *pa,
            Alloc::BuddyAlloc(alloc) => alloc.start_addr(),
        }
    }

    fn end_addr(&self) -> PAddr {
        match self {
            Alloc::BumpAlloc((pa, frame_count)) => *pa + PAGE_SIZE * frame_count,
            Alloc::BuddyAlloc(alloc) => alloc.end_addr(),
        }
    }
}

enum SendAllocator {
    Bump,
    Buddy(BuddyFrameAllocator),
}

impl SendAllocator {
    fn alloc(&self, frame_count: usize) -> Option<Alloc> {
        match self {
            Self::Bump => {
                mem::bump::alloc_frame(frame_count).map(|pa| Alloc::BumpAlloc((pa, frame_count)))
            }
            Self::Buddy(alloc) => alloc.alloc(frame_count).map(Alloc::BuddyAlloc),
        }
    }

    /// # Safety
    /// `alloc` must be an allocation from the currently active allocator.
    unsafe fn dealloc(&self, handle: &mut Alloc) {
        match self {
            Self::Bump => (),
            Self::Buddy(alloc) => {
                if let Alloc::BuddyAlloc(handle) = handle {
                    alloc.dealloc(handle);
                }
            }
        }
    }
}

unsafe impl Sync for SendAllocator {}

static ALLOCATOR: RwLock<SendAllocator> = RwLock::new(SendAllocator::Bump);

/// Should only be called once on higher-half phase
pub fn init(fdt: &Fdt, dtb_pa: PAddr) {
    let (mut usable_regs, _) = devicetree::get_region_slices(
        fdt,
        [
            // Safety: Used symbols are defined in the linker script
            unsafe {
                SizedMemoryRegion::new_unchecked(
                    kernel_start_p(),
                    kernel_end_p() - kernel_start_p(),
                )
            },
            // Safety: The size of the devicetree blob is nonzero
            unsafe {
                SizedMemoryRegion::new_unchecked(
                    dtb_pa,
                    fdt.total_size().next_multiple_of(PAGE_SIZE),
                )
            },
        ],
    );
    usable_regs.sort_unstable_by_key(|a| Reverse(a.size));
    for r in &usable_regs {
        println!("palloc : usable at 0x{:016x} .. 0x{:016x}", r.base, r.end());
    }

    let alloc = BuddyFrameAllocator::new(usable_regs.as_slice());
    *ALLOCATOR.write() = SendAllocator::Buddy(alloc);
}

pub fn alloc(frame_count: usize) -> Option<Alloc> {
    let a = ALLOCATOR.read();
    a.alloc(frame_count)
}

fn dealloc(handle: &mut Alloc) {
    let a = ALLOCATOR.read();
    // Safety: Bump-backed frames are never deallocated
    unsafe {
        a.dealloc(handle);
    }
}
