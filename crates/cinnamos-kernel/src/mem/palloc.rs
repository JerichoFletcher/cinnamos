use core::ptr::NonNull;

use fdt::Fdt;
use spin::Mutex;

use crate::{arch::PAddr, mem::{FrameAlloc, phys::{FrameAllocation, FrameAllocator, PhysFrameAllocator}, vms}, println};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PAllocError {
    AllocatorUninitialized,
}

struct SendAllocator(NonNull<FrameAllocator>);

unsafe impl Send for SendAllocator {}

pub struct Alloc(FrameAllocation);

impl Drop for Alloc {
    fn drop(&mut self) {
        dealloc(self);
    }
}

impl FrameAlloc for Alloc {
    fn base_addr(&self) -> PAddr {
        self.0.base_addr()
    }
}

static ALLOCATOR: Mutex<Option<SendAllocator>> = Mutex::new(None);

unsafe extern "C" {
    static KERNEL_START: PAddr;
    static KERNEL_END: PAddr;
}

pub fn init(fdt: &Fdt, dtb_ptr: *const u8) {
    for reg in fdt.memory().regions() {
        let start = PAddr::from_ptr(reg.starting_address);
        let end = start + reg.size.unwrap_or(0);

        if start < end {
            let mut base = start;
            unsafe {
                if start < KERNEL_START && KERNEL_END < end {
                    base = KERNEL_END;
                }
            }

            if end.addr() - base.addr() >= FrameAllocator::size(start, end) {
                let mut alloc_ptr = unsafe { FrameAllocator::create(base, start, end) };
                let alloc = unsafe { alloc_ptr.as_mut() };
                let alloc_paddr = PAddr::from_ptr(alloc_ptr.as_ptr().cast::<u8>());
                alloc.reserve(alloc_paddr, alloc_paddr + unsafe { core::mem::size_of_val_raw(alloc) });

                if let Some(rsv) = fdt.find_node("/reserved-memory") {
                    for n in rsv.children() {
                        if let Some(regs) = n.reg() {
                            for reg in regs {
                                if let Some(rsv_size) = reg.size {
                                    let rsv_start = PAddr::from_ptr(reg.starting_address);
                                    alloc.reserve(rsv_start, rsv_start + rsv_size);
                                }
                            }
                        }
                    }
                }

                for rsv in fdt.memory_reservations() {
                    let rsv_start = PAddr::from_ptr(rsv.address());
                    alloc.reserve(rsv_start, rsv_start + rsv.size());
                }

                let dtb_start = PAddr::from_ptr(dtb_ptr);
                alloc.reserve(dtb_start, dtb_start + fdt.total_size());

                *ALLOCATOR.lock() = Some(SendAllocator(alloc_ptr));
                println!("alloc : meta = 0x{:016x}; area = 0x{:016x}..0x{:016x}", alloc_ptr.addr(), start, end);
            } else {
                println!("Unable to create allocator for region 0x{:016x}..0x{:016x}", start, end);
            }
        }
    }
}

pub fn reinit_higher_half() -> Result<(), PAllocError> {
    let mut guard = ALLOCATOR.lock();
    let sa = guard.as_mut().ok_or(PAllocError::AllocatorUninitialized)?;
    
    let (ptr, ptr_meta) = sa.0.to_raw_parts();
    let pa = PAddr::from_ptr(ptr.as_ptr());
    let va = vms::phys_to_virt(pa);
    let ptr: *mut FrameAllocator = core::ptr::from_raw_parts_mut(va.as_mut::<()>(), ptr_meta);
    *guard = unsafe { Some(SendAllocator(NonNull::new_unchecked(ptr))) };

    Ok(())
}

pub fn alloc(size_bytes: usize) -> Option<Alloc> {
    let mut guard = ALLOCATOR.lock();
    let sa = guard.as_mut()?;
    unsafe { (*sa.0.as_ptr()).alloc(size_bytes).map(|v| Alloc(v)) }
}

pub fn dealloc(handle: &mut Alloc) {
    let mut guard = ALLOCATOR.lock();
    if let Some(sa) = guard.as_mut() {
        unsafe { (*sa.0.as_ptr()).dealloc(&mut handle.0); }
    }
}
