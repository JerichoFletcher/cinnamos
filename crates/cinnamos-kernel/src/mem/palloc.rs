use alloc::vec::Vec;
use fdt::Fdt;
use spin::Mutex;

use crate::{
    arch::{PAddr, VAddr},
    mem::{
        FrameAlloc, PAGE_SIZE, RegionSubtract, SizedMemoryRegion,
        phys::{FrameAllocation, FrameAllocator, PhysFrameAllocator},
        vms,
    },
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PAllocError {
    AllocatorUninitialized,
}

pub enum Alloc {
    BumpAlloc(PAddr),
    BuddyAlloc(FrameAllocation),
}

impl Drop for Alloc {
    fn drop(&mut self) {
        dealloc(self);
    }
}

impl FrameAlloc for Alloc {
    fn base_addr(&self) -> PAddr {
        match self {
            Alloc::BumpAlloc(pa) => *pa,
            Alloc::BuddyAlloc(alloc) => alloc.base_addr(),
        }
    }
}

enum SendAllocator {
    Bump,
    Buddy,
}

impl SendAllocator {
    fn alloc(&self, frame_count: usize) -> Option<Alloc> {
        match self {
            Self::Bump => mem::bump::alloc_frame(frame_count).map(|pa| Alloc::BumpAlloc(pa)),
            Self::Buddy => todo!(),
        }
    }

    /// # Safety
    /// `alloc` must be an allocation from the currently active allocator.
    unsafe fn dealloc(&self, _alloc: &mut Alloc) {
        match self {
            Self::Bump => (),
            Self::Buddy => todo!(),
        }
    }
}

// struct SendAllocator(NonNull<FrameAllocator>);

unsafe impl Send for SendAllocator {}

static ALLOCATOR: Mutex<Option<SendAllocator>> = Mutex::new(None);

/// Should only be called once on early phase
pub fn init_bump() {
    *ALLOCATOR.lock() = Some(SendAllocator::Bump);
}

pub fn init(fdt: &Fdt, dtb_ptr: *const u8) {
    let mut rsv_regs: Vec<SizedMemoryRegion> = fdt
        .memory_reservations()
        .map(|r| unsafe {
            SizedMemoryRegion::new_unchecked(PAddr::from_ptr(r.address()), r.size())
        })
        .chain(
            [
                // Safety: Used symbols are defined in the linker script
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        kernel_start_p!(),
                        kernel_end_p!() - kernel_start_p!(),
                    )
                },
                // Safety: The size of the devicetree blob is nonzero
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        vms::virt_to_phys(VAddr::from_ptr(dtb_ptr)),
                        (fdt.total_size() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1),
                    )
                },
            ]
            .into_iter(),
        )
        .collect();
    if let Some(rsv) = fdt.find_node("/reserved-memory") {
        rsv_regs.extend(
            rsv.children()
                .map(|n| n.reg())
                .filter_map(|r| {
                    r.map(|rs| {
                        rs.map(|r| {
                            SizedMemoryRegion::new(PAddr::from_ptr(r.starting_address), r.size)
                        })
                    })
                })
                .flatten()
                .filter_map(|r| r),
        );
    }
    rsv_regs.sort();

    let mut usable_regs: Vec<SizedMemoryRegion> = Vec::with_capacity(rsv_regs.len() + 1);
    for r in fdt
        .memory()
        .regions()
        .map(|r| SizedMemoryRegion::new(PAddr::from_ptr(r.starting_address), r.size))
        .filter_map(|r| r)
    {
        slice_usable_region(r, &mut rsv_regs, &mut usable_regs);
    }

    println!("Usable regions: {:?}", &usable_regs);
    todo!("Initialize allocators for each usable region");
}

pub fn alloc(frame_count: usize) -> Option<Alloc> {
    let mut guard = ALLOCATOR.lock();
    let sa = guard.as_mut()?;
    (*sa).alloc(frame_count)
}

pub fn dealloc(handle: &mut Alloc) {
    let mut guard = ALLOCATOR.lock();
    if let Some(sa) = guard.as_mut() {
        // Safety: Bump-backed frames are never deallocated
        unsafe {
            (*sa).dealloc(handle);
        }
    }
}

fn slice_usable_region(
    reg: SizedMemoryRegion,
    rsv: &mut [SizedMemoryRegion],
    out: &mut Vec<SizedMemoryRegion>,
) {
    rsv.sort_unstable();

    let mut reg = reg;
    for i in 0..rsv.len() {
        if reg.overlaps(&rsv[i]) {
            match reg.subtract(&rsv[i]) {
                RegionSubtract::None => return,
                RegionSubtract::Left(reg_l) => {
                    out.push(reg_l);
                    return;
                }
                RegionSubtract::Right(reg_r) => reg = reg_r,
                RegionSubtract::Both(reg_l, reg_r) => {
                    out.push(reg_l);
                    reg = reg_r;
                }
                RegionSubtract::Nonoverlapping => (),
            }
        }
    }
    out.push(reg);
}
