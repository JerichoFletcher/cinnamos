use core::mem::MaybeUninit;

use bitflags::bitflags;
use riscv::{register::satp};

use crate::{arch::{paddr::PAddr, vaddr::VAddr}, mem::{PhysFrameAlloc, addrsp::AddressSpace, physalloc::{self, Alloc}}};

pub const PAGE_SIZE: usize = 0x1000;
pub const PT_MAX_ENTRIES: usize = PAGE_SIZE / size_of::<PTE>();
pub const PAGE_TABLE_DEPTH: usize = PageSize::ALL.len();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    Page4K,
    Megapage2M,
    Gigapage1G,
    Terapage512G,
}

impl PageSize {
    pub const ALL: [Self; 4] = [
        Self::Page4K,
        Self::Megapage2M,
        Self::Gigapage1G,
        Self::Terapage512G,
    ];

    pub fn select_size(va: VAddr, pa: PAddr, size_bytes: usize) -> Option<Self> {
        let size_bytes = size_bytes.max(Self::Page4K.size());

        for i in (0..Self::ALL.len()).rev() {
            let s = Self::ALL[i];
            if s.size() > size_bytes {
                continue;
            }
    
            let low_mask = s.size() - 1;
            if va.addr() & low_mask == 0 && pa.addr() & low_mask == 0 {
                return Some(s)
            }
        }
        None
    }

    pub const fn size(&self) -> usize {
        match self {
            PageSize::Page4K => PAGE_SIZE,
            PageSize::Megapage2M => PAGE_SIZE << 9,
            PageSize::Gigapage1G => PAGE_SIZE << 18,
            PageSize::Terapage512G => PAGE_SIZE << 27,
        }
    }

    const fn level(&self) -> usize {
        match self {
            PageSize::Page4K => 0,
            PageSize::Megapage2M => 1,
            PageSize::Gigapage1G => 2,
            PageSize::Terapage512G => 3,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PTEFlags: u8 {
        const VALID = 0x01;
        const READ = 0x02;
        const WRITE = 0x04;
        const EXECUTE = 0x08;
        const USER = 0x10;
        const GLOBAL = 0x20;
        const ACCESSED = 0x40;
        const DIRTY = 0x80;

        const RW = 0x06;
        const RX = 0x0a;
        const RWX = 0x0e;
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PTE(usize);

impl PTE {
    pub const EMPTY: Self = Self(0);

    pub fn new(page_addr: PAddr, flags: PTEFlags) -> Self {
        debug_assert_eq!(page_addr.addr() & (PAGE_SIZE - 1), 0);
        let flags = flags.bits() as usize & 0xff;
        let paddr = (page_addr.addr() & 0xff_ffff_ffff_f000) >> 2;
        Self(paddr | flags)
    }

    pub fn phys_addr(&self) -> PAddr {
        PAddr::new(((self.0 << 10) as isize >> 8) as usize & 0xffff_ffff_ffff_f000)
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_retain((self.0 & 0xff) as u8)
    }

    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::VALID)
    }

    pub fn is_leaf(&self) -> bool {
        self.is_valid() && self.flags().intersects(PTEFlags::READ | PTEFlags::WRITE | PTEFlags::EXECUTE)
    }

    pub fn set_table(&mut self, pa: PAddr) {
        self.set(pa, PTEFlags::VALID);
    }

    pub fn set_leaf(&mut self, pa: PAddr, size: PageSize, flags: PTEFlags) {
        let mask = usize::MAX << (12 + size.level() * 9);
        let pa = PAddr::new(pa.addr() & mask);
        self.set(pa, flags | PTEFlags::VALID);
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    fn set(&mut self, page_addr: PAddr, flags: PTEFlags) {
        let flags = flags.bits() as usize & 0xff;
        let paddr = (page_addr.addr() & 0xff_ffff_ffff_f000) >> 2;
        self.0 = flags | paddr;
    }
}

#[repr(transparent)]
pub struct PageTable {
    pub entries: [PTE; PT_MAX_ENTRIES],
}

impl PageTable {
    /// # Safety
    /// `slot` must be dereferenceable to [PageTable](PageTable).
    pub unsafe fn init(slot: *mut MaybeUninit<Self>) -> *mut Self {
        if !slot.is_null() && slot.is_aligned() {
            // Safety: slot is not null and aligned
            unsafe { (*slot).write(Self { entries: [PTE::EMPTY; 512] }); }
        }
        slot.cast()
    }
}

pub enum PageTableAlloc {
    None,
    New(Alloc),
    Existing(*mut PageTable),
}

pub struct PageTableAllocMap {
    allocs: [PageTableAlloc; 3],
}

impl PageTableAllocMap {
    pub fn forget(self) {
        for v in self.allocs {
            core::mem::forget(v);
        }
    }

    pub fn take_new_allocs(self) -> impl Iterator<Item = Alloc> {
        core::iter::from_coroutine(#[coroutine] || {
            for v in self.allocs.into_iter().rev() {
                if let PageTableAlloc::New(alloc) = v {
                    yield alloc
                }
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    OutOfMemory,
    AlreadyMapped(VAddr, PAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
}

#[cfg(debug_assertions)]
pub fn translate_virt(root_pt: *mut PageTable, va: VAddr, p2v: impl Fn(PAddr) -> VAddr) -> Option<PAddr> {
    let vpn = va.vpn();
    let mut table = root_pt;

    for level in (0..=3).rev() {
        let pte = unsafe { &mut (*table).entries[vpn[level]] };
        let flags = pte.flags();

        if !pte.is_valid() || (!flags.contains(PTEFlags::READ) && flags.contains(PTEFlags::WRITE)) {
            return None
        } else if flags.intersects(PTEFlags::RX) {
            let pa = pte.phys_addr();
            let off_mask = (1usize << (12 + level * 9)) - 1;

            if pa.addr() & off_mask != 0 {
                return None
            } else {
                let pa = pa + (va.addr() & off_mask);
                return Some(pa)
            }
        } else {
            let next_pa = pte.phys_addr();
            table = p2v(next_pa).as_mut();
        }
    }
    None
}

pub fn map_page(
    root_pt: *mut PageTable,
    va: VAddr,
    pa: PAddr,
    size: PageSize,
    flags: PTEFlags,
    p2v: &impl Fn(PAddr) -> VAddr
) -> impl Iterator<Item = Result<Alloc, MapError>> {
    core::iter::from_coroutine(#[coroutine] move || {
        let vpn = va.vpn();
        let mut table = root_pt;
        
        for level in (0..=3).rev() {
            let pte = unsafe { &mut (*table).entries[vpn[level]] };
    
            if level == size.level() {
                if pte.is_valid() {
                    yield Err(MapError::AlreadyMapped(va, pte.phys_addr()));
                    return;
                }
                pte.set_leaf(pa, size, flags);
                return;
            } else {
                if pte.is_valid() && !pte.is_leaf() {
                    let next_pa = pte.phys_addr();
                    table = p2v(next_pa).as_mut();
                } else if !pte.is_valid() {
                    match physalloc::alloc(1) {
                        Some(alloc) => {
                            let next_pa = alloc.start_addr();
                            
                            let table_uninit = p2v(next_pa).as_mut::<MaybeUninit<_>>();
                            table = unsafe { PageTable::init(table_uninit) };
                            // crate::println!("SV48 alloc {:?}", &alloc);
            
                            pte.set_table(alloc.start_addr());
                            yield Ok(alloc);
                        },
                        None => {
                            yield Err(MapError::OutOfMemory);
                            return;
                        }
                    }
                } else {
                    yield Err(MapError::AlreadyMapped(va, pte.phys_addr()));
                    return;
                }
            }
        }
    })
}

pub fn unmap_page(root_pt: *mut PageTable, va: VAddr, p2v: impl Fn(PAddr) -> VAddr) -> Result<PageSize, UnmapError> {
    let vpn = va.vpn();
    let mut table = root_pt;

    for level in (0..=3).rev() {
        let pte = unsafe { &mut (*table).entries[vpn[level]] };
        if pte.is_valid() && !pte.is_leaf() {
            let next_pa = pte.phys_addr();
            table = p2v(next_pa).as_mut();
        } else if pte.is_valid() {
            pte.clear();
            riscv::asm::sfence_vma(0, va.addr());
            return Ok(PageSize::ALL[level])
        } else {
            return Err(UnmapError::NotMapped)
        }
    }
    unreachable!()
}

pub fn get_max_asid() -> usize {
    let mut satp = satp::read();
    let old_asid = satp.asid();
    satp.set_asid(usize::MAX);
    unsafe { satp::write(satp); }

    satp = satp::read();
    let max_asid = satp.asid();
    satp.set_asid(old_asid);
    unsafe { satp::write(satp); }

    max_asid
}

pub fn switch_address_space(addrsp: &AddressSpace) {
    let mut satp = satp::read();
    satp.set_ppn(addrsp.root_pa().ppn());
    satp.set_asid(addrsp.id());
    satp.set_mode(satp::Mode::Sv48);
    unsafe { satp::write(satp); }
}

pub fn flush_address_space(asid: usize) {
    riscv::asm::sfence_vma(asid, 0);
}
