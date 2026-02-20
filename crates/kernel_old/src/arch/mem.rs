use core::ptr::NonNull;
#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::mem;
#[cfg(target_arch = "riscv32")]
use mem::RiscvMem as MemImpl;

pub use mem::{
    VAddr,
    PAddr,
    PTEFlags,
    PTE,
    PT,
    PMode,
    PAGE_SIZE_ORD,
};

pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_ORD;

pub trait Mem {
    fn enable_paging(root: NonNull<PT>);

    fn map(root: NonNull<PT>, vaddr: VAddr, paddr: PAddr, flags: PTEFlags);
    fn unmap(root: NonNull<PT>);
    fn v_to_p(root: NonNull<PT>, vaddr: VAddr) -> Option<PAddr>;
}

#[inline(always)]
pub fn enable_paging(root: NonNull<PT>) {
    MemImpl::enable_paging(root);
}

pub fn map(root: NonNull<PT>, vaddr: VAddr, paddr: PAddr, flags: PTEFlags) {
    assert!(flags.intersects(PTEFlags::RWX), "At least one must be specified: R,W,X");
    if flags.contains(PTEFlags::Write) {
        assert!(flags.contains(PTEFlags::Read), "W specified without R");
    }
    MemImpl::map(root, vaddr, paddr, flags);
}

#[inline(always)]
pub fn unmap(root: NonNull<PT>) {
    MemImpl::unmap(root);
}

#[inline(always)]
pub fn v_to_p(root: NonNull<PT>, vaddr: VAddr) -> Option<PAddr> {
    MemImpl::v_to_p(root, vaddr)
}
