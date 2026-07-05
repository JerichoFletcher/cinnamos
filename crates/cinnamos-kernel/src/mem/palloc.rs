use core::mem;

use fdt::Fdt;
use spin::Mutex;

use crate::{arch::paddr::PAddr, mem::allocator::{FreeListAllocator, FreeListFrameAlloc, PhysFrameAllocator}, println};

struct SendAllocator(*mut FreeListAllocator);

unsafe impl Send for SendAllocator {}

static ALLOCATOR: Mutex<Option<SendAllocator>> = Mutex::new(None);

unsafe extern "C" {
    static KERNEL_START: usize;
    static KERNEL_END: usize;
}

pub fn init(fdt: &Fdt) {
    for reg in fdt.memory().regions() {
        let start = PAddr::from_ptr(reg.starting_address);
        let end = start + reg.size.unwrap_or(0);

        if start < end {
            let mut base = start;
            unsafe {
                if start.addr() < KERNEL_START && KERNEL_END < end.addr() {
                    base = PAddr::new(KERNEL_END);
                }
            }

            if end.addr() - base.addr() >= FreeListAllocator::size(start, end) {
                let alloc = unsafe { FreeListAllocator::create(base, start, end) };
                let alloc_paddr = PAddr::from_ptr(alloc.cast());
                unsafe { (*alloc).reserve(alloc_paddr, alloc_paddr + mem::size_of_val_raw(alloc)); }

                if let Some(rsv) = fdt.find_node("/reserved-memory") {
                    for n in rsv.children() {
                        if let Some(regs) = n.reg() {
                            for reg in regs {
                                if let Some(rsv_size) = reg.size {
                                    let rsv_start = PAddr::from_ptr(reg.starting_address);
                                    unsafe { (*alloc).reserve(rsv_start, rsv_start + rsv_size); }
                                }
                            }
                        }
                    }
                }

                for rsv in fdt.memory_reservations() {
                    let rsv_start = PAddr::from_ptr(rsv.address());
                    unsafe { (*alloc).reserve(rsv_start, rsv_start + rsv.size()); }
                }

                *ALLOCATOR.lock() = Some(SendAllocator(alloc));
                println!("alloc : meta = 0x{:016x}; area = 0x{:016x}..0x{:016x}", alloc.addr(), start.addr(), end.addr());
            } else {
                println!("Unable to create allocator for region 0x{:016x}..0x{:016x}", start.addr(), end.addr());
            }
        }
    }
}

pub fn alloc(size_bytes: usize) -> Option<FreeListFrameAlloc> {
    let mut l = ALLOCATOR.lock();
    let sa = l.as_mut()?;
    unsafe { (*sa.0).alloc(size_bytes) }
}

pub fn dealloc(handle: FreeListFrameAlloc) {
    let mut l = ALLOCATOR.lock();
    if let Some(sa) = l.as_mut() {
        unsafe { (*sa.0).dealloc(handle); }
    }
}
