use generic_once_cell::OnceCell;

use crate::lock::RawSpinLock;
use crate::mem::{PAGE_ALIGN_ORD, PhysAddr};

unsafe extern "C" {
    static KERNEL_PHYS_START: PhysAddr;
    static KERNEL_PHYS_END: PhysAddr;
}

#[derive(Clone, Copy, Debug)]
struct MemoryRegion {
    pub start: PhysAddr,
    pub end: PhysAddr,
}

static MEM_SPACE: OnceCell<RawSpinLock, MemoryRegion> = OnceCell::new();

pub fn init(mem_start: PhysAddr, size: usize) {
    let kernel_phys_start = unsafe { KERNEL_PHYS_START };
    let kernel_phys_end = unsafe { KERNEL_PHYS_END };

    let mut mem_start = mem_start;
    if mem_start.0 < kernel_phys_end.0 {
        mem_start = kernel_phys_end.align_next(PAGE_ALIGN_ORD);
    }

    if kernel_phys_start.0 <= mem_start.0 && mem_start.0 < kernel_phys_end.0 {
        mem_start = kernel_phys_end;
    }

    let mut mem_end = mem_start.add(size);
    if kernel_phys_start.0 <= mem_end.0 && mem_end.0 < kernel_phys_end.0 {
        mem_end = kernel_phys_start;
    }

    MEM_SPACE.set(MemoryRegion { start: mem_start, end: mem_end }).expect("Region already initialized");
}
