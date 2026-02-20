use crate::arch::riscv32::vms::sv32;

pub use sv32::{
    PhysAddr,
    VirtAddr,
    PAGE_ALIGN_ORD,
    init_boot_pt
};

pub const PAGE_SIZE: usize = 1 << PAGE_ALIGN_ORD;
