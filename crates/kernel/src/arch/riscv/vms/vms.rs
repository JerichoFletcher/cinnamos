use deranged::RangedUsize;
use crate::arch::mem::{PAddr, PTEFlags, PT};

pub trait PageTableEntry {
    fn is_valid(&self) -> bool;
    fn is_leaf(&self) -> bool;
    fn set_flags(&mut self, flags: PTEFlags);

    fn get_paddr(&self) -> PAddr;
    fn set_paddr(&mut self, ppn: PAddr);
}

pub trait PageTable {
    fn len() -> usize;
}

pub trait PagingMode<const LEVEL: usize, VAddr : Clone, PAddr : Clone> {
    const PT_ENTRY_COUNT: usize;
    const MAX_LEVELS: usize = LEVEL;

    fn enable_paging(root: &PT);

    fn v_to_p(vaddr: VAddr, pte_paddr: PAddr, level: RangedUsize<0, LEVEL>) -> PAddr;

    fn vpn_index(vaddr: VAddr, level: RangedUsize<0, LEVEL>) -> usize;
    // fn ppn_index(paddr: PAddr, level: RangedUsize<0, LEVEL>) -> usize;

    fn vpn_indices(vaddr: VAddr) -> [usize; LEVEL] {
        let mut vpn = [0usize; LEVEL];
        for i in 0..LEVEL {
            vpn[i] = Self::vpn_index(vaddr.clone(), RangedUsize::new(i).unwrap());
        }
        vpn
    }

    // fn ppn_indices(paddr: PAddr) -> [usize; LEVEL] {
    //     let mut ppn = [0usize; LEVEL];
    //     for i in 0..LEVEL {
    //         ppn[i] = Self::ppn_index(paddr.clone(), RangedUsize::new(i).unwrap());
    //     }
    //     ppn
    // }
}
