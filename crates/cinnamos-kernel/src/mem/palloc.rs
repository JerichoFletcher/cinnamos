use fdt::Fdt;
use spin::Mutex;

use crate::{
    arch::PAddr,
    mem::{
        PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion,
        phys::{
            PhysFrameAllocator,
            buddy::{BuddyFrameAlloc, BuddyFrameAllocator},
        },
    },
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PAllocError {
    AllocatorUninitialized,
}

#[derive(Debug)]
pub enum Alloc {
    BumpAlloc(PAddr),
    BuddyAlloc(BuddyFrameAlloc),
}

impl Drop for Alloc {
    fn drop(&mut self) {
        dealloc(self);
    }
}

impl PhysFrameAlloc for Alloc {
    fn base_addr(&self) -> PAddr {
        match self {
            Alloc::BumpAlloc(pa) => *pa,
            Alloc::BuddyAlloc(alloc) => alloc.base_addr(),
        }
    }
}

enum SendAllocator<'a> {
    Bump,
    Buddy(BuddyFrameAllocator<'a>),
}

impl<'a> SendAllocator<'a> {
    fn alloc(&mut self, frame_count: usize) -> Option<Alloc> {
        match self {
            Self::Bump => mem::bump::alloc_frame(frame_count).map(|pa| Alloc::BumpAlloc(pa)),
            Self::Buddy(alloc) => alloc.alloc(frame_count).map(|a| Alloc::BuddyAlloc(a)),
        }
    }

    /// # Safety
    /// `alloc` must be an allocation from the currently active allocator.
    unsafe fn dealloc(&mut self, handle: &mut Alloc) {
        match self {
            Self::Bump => (),
            Self::Buddy(alloc) => {
                if let Alloc::BuddyAlloc(handle) = handle {
                    alloc.dealloc(handle);
                } else {
                    panic!("Invalid handle for current allocator: {:?}", handle);
                }
            }
        }
    }
}

// struct SendAllocator(NonNull<FrameAllocator>);

unsafe impl Send for SendAllocator<'_> {}

static ALLOCATOR: Mutex<Option<SendAllocator>> = Mutex::new(None);

/// Should only be called once on early phase
pub fn init_bump() {
    *ALLOCATOR.lock() = Some(SendAllocator::Bump);
}

/// Should only be called once on higher-half phase
pub fn init(fdt: &Fdt, dtb_pa: PAddr) {
    let (mut usable_regs, _) = devicetree::get_region_slices(
        fdt,
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
                    dtb_pa,
                    (fdt.total_size() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1),
                )
            },
        ],
    );
    usable_regs.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    println!("USABLE {:?}", &usable_regs);

    let mut alloc = BuddyFrameAllocator::new(&[usable_regs[0]]);
    let (_, regs) = usable_regs.split_at(1);
    for reg in regs {
        alloc.add_region(reg);
    }

    *ALLOCATOR.lock() = Some(SendAllocator::Buddy(alloc));
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
