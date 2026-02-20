pub mod sv32;

use sv32 as sv;
pub use sv::{
    VAddr,
    PAddr,
    PMode,
    PTEFlags,
    PTE,
    PT,
};
pub const PAGE_SIZE_ORD: usize = 12; // 4096
pub const PAGE_LOW_MASK: usize = (1 << PAGE_SIZE_ORD) - 1;

mod vms;
pub use vms::*;
