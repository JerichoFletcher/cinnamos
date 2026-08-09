use core::{cmp::Reverse, num::NonZero};

use alloc::boxed::Box;
use fdt::Fdt;
use spin::RwLock;

use crate::{
    arch::PAddr,
    mem::{
        PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion,
        alloc::bump::BumpFrameAlloc,
        phys::{
            PhysFrameAllocator,
            buddy::{BuddyFrameAlloc, BuddyFrameAllocator},
        },
    },
    sym::*,
    *,
};

/// A generalized physical frame allocation handed out by [`physalloc`](crate::mem::physalloc).
#[derive(Debug)]
pub enum FrameAlloc {
    /// This allocation comes from the bump allocator.
    BumpAlloc(BumpFrameAlloc),
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
            FrameAlloc::BumpAlloc(alloc) => alloc.start_addr(),
            FrameAlloc::BuddyAlloc(alloc) => alloc.start_addr(),
        }
    }

    fn end_addr(&self) -> PAddr {
        match self {
            FrameAlloc::BumpAlloc(alloc) => alloc.end_addr(),
            FrameAlloc::BuddyAlloc(alloc) => alloc.end_addr(),
        }
    }

    fn size(&self) -> usize {
        match self {
            FrameAlloc::BumpAlloc(alloc) => alloc.size(),
            FrameAlloc::BuddyAlloc(alloc) => alloc.size(),
        }
    }

    fn frame_count(&self) -> NonZero<usize> {
        match self {
            FrameAlloc::BumpAlloc(alloc) => alloc.frame_count(),
            FrameAlloc::BuddyAlloc(alloc) => alloc.frame_count(),
        }
    }
}

enum SendAllocator {
    Bump,
    Buddy(Box<BuddyFrameAllocator<'static>>),
}

impl SendAllocator {
    /// Allocates a number of physical frames.
    /// If allocation fails, this function returns [`None`].
    fn alloc(&self, frame_count: usize) -> Option<FrameAlloc> {
        match self {
            Self::Bump => mem::alloc::bump::alloc_frame(frame_count).map(FrameAlloc::BumpAlloc),
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
    let mut g = ALLOCATOR.write();
    if !matches!(*g, SendAllocator::Buddy(_)) {
        let mut alloc = BuddyFrameAllocator::new();
        let (mut usable_regs, _) = devicetree::get_region_slices(
            fdt,
            [
                // Safety: Used symbols are defined in the linker script
                unsafe { SizedMemoryRegion::new_unchecked(kernel_start_p(), kernel_size()) },
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
            log::debug!("usable at {:#016x} .. {:#016x}", r.base, r.end());
            // Safety: get_region_slices guarantees disjoint usable regions
            unsafe { alloc.add_region(r) };
        }

        *g = SendAllocator::Buddy(Box::new(alloc));
    }
}

/// Adds a memory region to the frame allocator.
///
/// This function only adds the region when the buddy allocator is initialized.
/// Otherwise, it does nothing.
///
/// # Safety
/// The added region must not intersect with any existing regions managed by the allocator.
pub unsafe fn add_region(reg: &SizedMemoryRegion) {
    let mut alloc = ALLOCATOR.write();
    if let SendAllocator::Buddy(alloc) = &mut *alloc {
        // Safety: reg does not intersect with any existing regions
        unsafe { alloc.add_region(reg) };
    }
}

/// Requests a number of physical frames to be allocated.
/// If allocation fails, this function returns [`None`].
pub fn alloc(frame_count: usize) -> Option<FrameAlloc> {
    let a = ALLOCATOR.read().alloc(frame_count);
    match &a {
        Some(a) => log::trace!(
            "ppage size={} base={:#016x} alloc",
            frame_count * PAGE_SIZE,
            &a.start_addr()
        ),
        None => log::trace!("ppage size={} failed alloc", frame_count * PAGE_SIZE),
    }
    a
}

/// Releases a physical frame allocation.
fn dealloc(handle: &mut FrameAlloc) {
    log::trace!(
        "ppage size={} base={:#016x} dealloc",
        handle.frame_count().get() * PAGE_SIZE,
        handle.start_addr(),
    );
    let a = ALLOCATOR.read();
    a.dealloc(handle);
}
