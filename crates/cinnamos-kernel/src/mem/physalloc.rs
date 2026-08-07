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

/// A physical frame allocation.
#[derive(Debug)]
pub enum FrameAlloc {
    /// This allocation comes from the bump allocator.
    BumpAlloc((PAddr, usize)),
    /// This allocation comes from the dedicated buddy allocator.
    BuddyAlloc(BuddyFrameAlloc),
}

impl Drop for FrameAlloc {
    fn drop(&mut self) {
        dealloc(self);
    }
}

impl PhysFrameAlloc for FrameAlloc {
    fn start_addr(&self) -> PAddr {
        match self {
            FrameAlloc::BumpAlloc((pa, _)) => *pa,
            FrameAlloc::BuddyAlloc(alloc) => alloc.start_addr(),
        }
    }

    fn end_addr(&self) -> PAddr {
        match self {
            FrameAlloc::BumpAlloc((pa, frame_count)) => *pa + PAGE_SIZE * frame_count,
            FrameAlloc::BuddyAlloc(alloc) => alloc.end_addr(),
        }
    }

    fn frame_count(&self) -> usize {
        match self {
            FrameAlloc::BumpAlloc((_, frame_count)) => *frame_count,
            FrameAlloc::BuddyAlloc(alloc) => alloc.frame_count(),
        }
    }
}

enum SendAllocator {
    Bump,
    Buddy(BuddyFrameAllocator),
}

impl SendAllocator {
    /// Allocates a number of physical frames.
    /// If allocation fails, this function returns [`None`].
    fn alloc(&self, frame_count: usize) -> Option<FrameAlloc> {
        match self {
            Self::Bump => mem::alloc::bump::alloc_frame(frame_count)
                .map(|pa| FrameAlloc::BumpAlloc((pa, frame_count))),
            Self::Buddy(alloc) => alloc.alloc(frame_count).map(FrameAlloc::BuddyAlloc),
        }
    }

    /// Deallocates a previously created physical frame allocation.
    fn dealloc(&self, handle: &mut FrameAlloc) {
        match self {
            Self::Bump => (),
            Self::Buddy(alloc) => {
                if let FrameAlloc::BuddyAlloc(handle) = handle {
                    alloc.dealloc(handle);
                }
            }
        }
    }
}

static ALLOCATOR: RwLock<SendAllocator> = RwLock::new(SendAllocator::Bump);

/// Initializes the buddy allocator. This function scans the devicetree for usable memory regions
/// and adds them to the allocator.
///
/// Should only be called once on higher-half phase.
pub fn init(fdt: &Fdt, dtb_pa: PAddr) {
    let (mut usable_regs, _) = devicetree::get_region_slices(
        fdt,
        [
            // Safety: Used symbols are defined in the linker script
            unsafe { SizedMemoryRegion::new_unchecked(kernel_start_p(), kernel_size()) },
            // Safety: Used symbols are defined in the linker script
            unsafe { SizedMemoryRegion::new_unchecked(bump_heap_start_p(), bump_heap_size()) },
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
        log::info!("usable at {:#016x} .. {:#016x}", r.base, r.end());
    }

    let alloc = BuddyFrameAllocator::new(usable_regs.as_slice());
    *ALLOCATOR.write() = SendAllocator::Buddy(alloc);
}

/// Adds a memory region to the frame allocator.
///
/// This function only adds the region when the buddy allocator is initialized.
/// Otherwise, it does nothing.
pub fn add_region(reg: &SizedMemoryRegion) {
    let mut alloc = ALLOCATOR.write();
    if let SendAllocator::Buddy(alloc) = &mut *alloc {
        alloc.add_region(reg);
    }
}

/// Requests a number of physical frames to be allocated.
/// If allocation fails, this function returns [`None`].
pub fn alloc(frame_count: usize) -> Option<FrameAlloc> {
    let a = ALLOCATOR.read().alloc(frame_count);
    match &a {
        Some(a) => log::trace!(
            "allocate ppage size={} base={:#016x}",
            frame_count * PAGE_SIZE,
            &a.start_addr()
        ),
        None => log::trace!("allocate ppage size={} failed", frame_count * PAGE_SIZE),
    }
    a
}

/// Releases a physical frame allocation.
fn dealloc(handle: &mut FrameAlloc) {
    let a = ALLOCATOR.read();
    a.dealloc(handle);
}
