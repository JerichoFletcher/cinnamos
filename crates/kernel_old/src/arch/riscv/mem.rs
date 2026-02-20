use core::ptr::NonNull;
use deranged::RangedUsize;
use crate::arch::riscv::vms;
pub use vms::{
    VAddr,
    PAddr,
    PTEFlags,
    PTE,
    PT,
    PMode,
    PAGE_SIZE_ORD,
    PAGE_LOW_MASK,
};

use vms::{PageTable, PageTableEntry, PagingMode};
use crate::arch::mem::Mem;
use crate::page::{dealloc, zalloc};

pub struct RiscvMem;

impl Mem for RiscvMem {
    #[inline(always)]
    fn enable_paging(root: NonNull<PT>) {
        PMode::enable_paging(root, 0);
    }

    fn map(mut root: NonNull<PT>, vaddr: VAddr, paddr: PAddr, flags: PTEFlags) {
        let vpn = PMode::vpn_indices(vaddr);

        let mut v = unsafe {
            &mut root.as_mut().entries[vpn[PMode::MAX_LEVELS - 1]]
        };
        for i in (1..PMode::MAX_LEVELS).rev() {
            if !v.is_valid() {
                match zalloc(1) {
                    Some(ptr) => {
                        v.set_paddr(ptr.as_ptr() as PAddr);
                        v.set_valid(true);
                    }
                    None => panic!("Failed to allocate page"),
                }
            }
            let entry = v.get_paddr() as *mut PTE;
            v = unsafe { entry.add(vpn[i - 1]).as_mut().unwrap() };
        }

        v.set_paddr(paddr);
        v.set_flags(flags);
        v.set_valid(true);
    }

    fn unmap(root: NonNull<PT>) {
        for lv2 in 0..PT::len() {
            let entry_lv2 = unsafe { &root.as_ref().entries[lv2] };
            if entry_lv2.is_valid() && !entry_lv2.is_leaf() {
                let addr_lv1 = entry_lv2.get_paddr() as *mut PT;
                let table_lv1 = unsafe { addr_lv1.as_mut().unwrap() };

                for lv1 in 0..PT::len() {
                    let ref entry_lv1 = table_lv1.entries[lv1];
                    if entry_lv1.is_valid() && !entry_lv1.is_leaf() {
                        let addr_lv0 = entry_lv1.get_paddr() as *mut PT;
                        dealloc(NonNull::new(addr_lv0.cast::<u8>()).unwrap());
                    }
                }
                dealloc(NonNull::new(addr_lv1.cast::<u8>()).unwrap());
            }
        }
    }

    fn v_to_p(root: NonNull<PT>, vaddr: VAddr) -> Option<PAddr> {
        let vpn = PMode::vpn_indices(vaddr);
        let mut v = unsafe { &root.as_ref().entries[vpn[PMode::MAX_LEVELS - 1]] };

        for i in (0..PMode::MAX_LEVELS).rev() {
            if !v.is_valid() {
                break;
            } else if v.is_leaf() {
                return Some(PMode::v_to_p(vaddr, v.get_paddr(), RangedUsize::new(i).unwrap()));
            }

            if i > 0 {
                let entry = v.get_paddr() as *const PTE;
                v = unsafe { entry.add(vpn[i - 1]).as_ref().unwrap() };
            }
        }
        None
    }
}
