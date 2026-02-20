pub type VAddr = u32;
pub type PAddr = u64;

use core::ptr::NonNull;
pub use Sv32 as PMode;
pub use Sv32PTEPubFlags as PTEFlags;
pub use Sv32PTE as PTE;
pub use Sv32PT as PT;

use bitflags::bitflags;
use deranged::RangedUsize;
use riscv::register::satp;
use crate::arch::mem::PAGE_SIZE_ORD;
use crate::arch::riscv::vms::{PageTable, PageTableEntry, PagingMode, PAGE_LOW_MASK};

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct Sv32PTEFlags: u8 {
        const Valid = 1 << 0;
        const Read = 1 << 1;
        const Write = 1 << 2;
        const Execute = 1 << 3;
        const User = 1 << 4;
        const Global = 1 << 5;
        const Accessed = 1 << 6;
        const Dirty = 1 << 7;

        const RWX = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
        const RWXU = Self::RWX.bits() | Self::User.bits();
    }
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct Sv32PTEPubFlags: u8 {
        const Read = 1 << 1;
        const Write = 1 << 2;
        const Execute = 1 << 3;
        const User = 1 << 4;
        const Global = 1 << 5;

        const ReadWrite = Self::Read.bits() | Self::Write.bits();
        const ReadExecute = Self::Read.bits() | Self::Execute.bits();
        const RWX = Self::Read.bits() | Self::Write.bits() | Self::Execute.bits();
    }
}

impl Into<Sv32PTEFlags> for Sv32PTEPubFlags {
    fn into(self) -> Sv32PTEFlags {
        Sv32PTEFlags::from_bits_truncate(self.bits())
    }
}

#[repr(C)]
pub struct Sv32PTE {
    pub bits: u32,
}

impl PageTableEntry for Sv32PTE {
    #[inline]
    fn is_leaf(&self) -> bool {
        self.flags().intersects(Sv32PTEFlags::RWX)
    }

    #[inline]
    fn set_flags(&mut self, flags: PTEFlags) {
        let sh = self.bits & !0xff;
        let mut pteflags = self.flags();
        pteflags.insert(flags.into());
        self.bits = sh | pteflags.bits() as u32;
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.flags().contains(Sv32PTEFlags::Valid)
    }

    fn set_valid(&mut self, valid: bool) {
        let sh = self.bits & !0xff;
        let mut pteflags = self.flags();
        pteflags.set(Sv32PTEFlags::Valid, valid);
        self.bits = sh | pteflags.bits() as u32;
    }

    #[inline]
    fn get_paddr(&self) -> PAddr {
        (self.bits as PAddr) << 2 & !PAGE_LOW_MASK as PAddr
    }

    #[inline]
    fn set_paddr(&mut self, ppn: PAddr) {
        let sh = (ppn as usize & !PAGE_LOW_MASK) >> 2;
        self.bits = (sh | (self.bits as usize & PAGE_LOW_MASK)) as u32;
    }
}

impl Sv32PTE {
    #[inline(always)]
    pub fn flags(&self) -> Sv32PTEFlags {
        Sv32PTEFlags::from_bits((self.bits & 0xff) as u8).unwrap()
    }
}

pub struct Sv32PT {
    pub entries: [Sv32PTE; Sv32::PT_ENTRY_COUNT],
}

impl PageTable for Sv32PT {
    #[inline(always)]
    fn len() -> usize {
        Sv32::PT_ENTRY_COUNT
    }
}

pub struct Sv32;

impl PagingMode<2, VAddr, PAddr> for Sv32 {
    const PT_ENTRY_COUNT: usize = 1024;
    const ASID_LENGTH: usize = 9;

    fn enable_paging(root: NonNull<PT>, asid: usize) {
        let mut atp = satp::Satp::from_bits(0);
        let asid_mask = (1 << Self::ASID_LENGTH) - 1;
        let asid = asid & asid_mask;

        atp.set_mode(satp::Mode::Sv32);
        atp.set_asid(asid);
        atp.set_ppn(root.as_ptr() as usize >> PAGE_SIZE_ORD);
        unsafe { satp::write(atp); }
        riscv::asm::sfence_vma(asid, 0);
    }

    fn v_to_p(vaddr: VAddr, pte_paddr: PAddr, level: RangedUsize<0, 2>) -> PAddr {
        let mask = (1 << (12 + level.get() * 10)) - 1;
        let vaddr_lower = vaddr & (mask as VAddr);
        let paddr_upper = pte_paddr & !(mask as PAddr);
        paddr_upper | (vaddr_lower as PAddr)
    }

    fn vpn_index(vaddr: VAddr, level: RangedUsize<0, 2>) -> usize {
        match level.get() {
            0 => ((vaddr >> 12) & 0x3ff) as usize,
            1 => ((vaddr >> 22) & 0x3ff) as usize,
            _ => panic!("Invalid level {}", level),
        }
    }

    // fn ppn_index(paddr: PAddr, level: RangedUsize<0, 2>) -> usize {
    //     match level.get() {
    //         0 => ((paddr >> 12) & 0x3ff) as usize,
    //         1 => ((paddr >> 22) & 0xfff) as usize,
    //         _ => panic!("Invalid level {}", level),
    //     }
    // }
}
