use bitflags::bitflags;
use crate::{arch::riscv32::vms::PAGE_SIZE, bits};

pub const PAGE_ALIGN_ORD: u8 = 12;
pub const PAGE_LOW_MASK: usize = (1 << PAGE_ALIGN_ORD) - 1;
pub const PT_ENTRY_COUNT: usize = 1024;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    #[inline(always)]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    #[inline(always)]
    pub const fn from_usize(addr: usize) -> Self {
        Self(addr as u64)
    }

    #[inline(always)]
    pub fn from_ptr(ptr: *const u8) -> Self {
        Self(ptr as u64)
    }

    #[inline(always)]
    pub fn add(&self, offset: usize) -> Self {
        Self::new(self.0 + offset as u64)
    }

    #[inline(always)]
    pub fn align_next(&self, order: u8) -> Self {
        Self::new(bits::align_next_u64(self.0, order))
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub u32);

impl VirtAddr {
    #[inline(always)]
    pub const fn new(addr: u32) -> Self {
        Self(addr)
    }
}

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

impl From<Sv32PTEPubFlags> for Sv32PTEFlags {
    fn from(flags: Sv32PTEPubFlags) -> Self {
        Sv32PTEFlags::from_bits_truncate(flags.bits())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sv32PTE {
    pub bits: u32,
}

///////////////////////// BOOT /////////////////////////
#[repr(align(4096))]
struct StaticBootPT {
    root: [u32; PT_ENTRY_COUNT],
    l0_id: [u32; PT_ENTRY_COUNT],
    l0_hi: [u32; PT_ENTRY_COUNT],
}

#[unsafe(link_section = ".data.boot")]
static mut BOOT_PT: StaticBootPT = StaticBootPT {
    root: [0; PT_ENTRY_COUNT],
    l0_id: [0; PT_ENTRY_COUNT],
    l0_hi: [0; PT_ENTRY_COUNT],
};

#[unsafe(link_section = ".text.boot")]
unsafe fn setup_boot_pt(dtb_ptr: *const u8) -> usize {
    unsafe {
        unsafe extern "C" {
            static BOOT_START: PhysAddr;
            static BOOT_END: PhysAddr;
            static KERNEL_START: PhysAddr;
            static KERNEL_END: PhysAddr;
        }

        let l0_id_phys = &raw const BOOT_PT.l0_id as *const _ as usize;
        let l0_hi_phys = &raw const BOOT_PT.l0_hi as *const _ as usize;

        let vpn1_id = (BOOT_START.0 >> 22) & 0x3ff;
        let vpn1_hi = (KERNEL_START.0 >> 22) & 0x3ff;

        BOOT_PT.root[vpn1_id as usize] = ((l0_id_phys >> 12) << 10) as u32 | 0b1;
        BOOT_PT.root[vpn1_hi as usize] = ((l0_hi_phys >> 12) << 10) as u32 | 0b1;

        let flags = 0b0010_1111;
        let mut paddr = BOOT_START;
        let mut vpn0 = ((BOOT_START.0 >> 12) & 0x3ff) as usize;
        while paddr.0 < BOOT_END.0 && vpn0 < PT_ENTRY_COUNT {
            let ppn0 = (paddr.0 >> 12) as u32;
            let pte = (ppn0 << 10) | flags;
            BOOT_PT.l0_id[vpn0] = pte;

            paddr = paddr.add(PAGE_SIZE);
            vpn0 += 1;
        }

        let kernel_vsize = (KERNEL_END.0 - KERNEL_START.0) as usize;
        let kernel_page_ct = bits::div_ceil(kernel_vsize, PAGE_SIZE);
        let mut paddr = BOOT_START;
        let mut vpn0 = ((KERNEL_START.0 >> 12) & 0x3ff) as usize;
        let end_vpn0 = vpn0 + kernel_page_ct;
        while vpn0 < end_vpn0 && vpn0 < PT_ENTRY_COUNT {
            let ppn0 = (paddr.0 >> 12) as u32;
            let pte = (ppn0 << 10) | flags;
            BOOT_PT.l0_hi[vpn0] = pte;

            paddr = paddr.add(PAGE_SIZE);
            vpn0 += 1;
        }

        let fdt_paddr = dtb_ptr as usize;
        let vpn1_fdt = ((fdt_paddr >> 22) & 0x3ff) as u32;
        let ppn1_fdt = ((fdt_paddr >> 22) & 0xfff) as u32;
        BOOT_PT.root[vpn1_fdt as usize] = (ppn1_fdt << 20) | flags;

        &raw const BOOT_PT.root as *const _ as usize
    }
}

#[unsafe(link_section = ".text.boot")]
pub fn init_boot_pt(dtb_ptr: *const u8) -> usize {
    unsafe { setup_boot_pt(dtb_ptr) }
}
