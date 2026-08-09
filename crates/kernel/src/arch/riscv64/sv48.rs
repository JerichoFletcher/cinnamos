use core::mem::MaybeUninit;

use bitflags::bitflags;
use riscv::register::satp;

use crate::{
    arch::{paddr::PAddr, vaddr::VAddr},
    mem::{
        PhysFrameAlloc,
        addrsp::AddressSpace,
        physalloc::{self, FrameAlloc},
    },
};

/// The size of a page, in bytes.
pub const PAGE_SIZE: usize = 0x1000;
/// The maximum number of entries in a [`PageTable`].
pub const PT_MAX_ENTRIES: usize = PAGE_SIZE / size_of::<PTE>();
/// The maximum page table depth used by the current protocol.
pub const PAGE_TABLE_DEPTH: usize = PageLevel::ALL.len();

/// The supported page levels that can be mapped with a single [`PTE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageLevel {
    /// A page of size 4 KiB.
    Page4K,
    /// A megapage of size 2 MiB.
    Megapage2M,
    /// A gigapage of size 1 GiB.
    Gigapage1G,
    /// A terapage of size 512 GiB.
    Terapage512G,
}

impl PageLevel {
    /// An array containing all available page levels.
    pub const ALL: [Self; 4] = [
        Self::Page4K,
        Self::Megapage2M,
        Self::Gigapage1G,
        Self::Terapage512G,
    ];

    /// Returns the largest [`PageLevel`] such that a page of such level fulfills these criteria:
    /// - The page size is greater than or equal to `size_bytes`.
    /// - The base address aligns with both `va` and `pa`.
    ///
    /// If no such `PageLevel` exists, the function returns [`None`](None).
    pub fn select_size(va: VAddr, pa: PAddr, size_bytes: usize) -> Option<Self> {
        let size_bytes = size_bytes.max(Self::Page4K.size());

        for i in (0..Self::ALL.len()).rev() {
            let s = Self::ALL[i];
            if s.size() > size_bytes {
                continue;
            }

            let low_mask = s.size() - 1;
            if va.addr() & low_mask == 0 && pa.addr() & low_mask == 0 {
                return Some(s);
            }
        }
        None
    }

    /// Checks whether an address has sufficient alignment for this level.
    pub const fn is_aligned(&self, addr: usize) -> bool {
        addr & (self.size() - 1) == 0
    }

    /// The corresponding size, in bytes.
    pub const fn size(&self) -> usize {
        match self {
            PageLevel::Page4K => PAGE_SIZE,
            PageLevel::Megapage2M => PAGE_SIZE << 9,
            PageLevel::Gigapage1G => PAGE_SIZE << 18,
            PageLevel::Terapage512G => PAGE_SIZE << 27,
        }
    }

    /// The corresponding level, in bytes.
    const fn level(&self) -> usize {
        match self {
            PageLevel::Page4K => 0,
            PageLevel::Megapage2M => 1,
            PageLevel::Gigapage1G => 2,
            PageLevel::Terapage512G => 3,
        }
    }
}

bitflags! {
    /// Various flags that can be associated with a [`PTE`](PTE).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PTEFlags: u8 {
        /// Marks a [`PTE`] as representing a valid mapping.
        const VALID     = 0x01;
        /// Marks a [`PTE`] mapping as a readable leaf.
        const READ      = 0x02;
        /// Marks a [`PTE`] mapping as a writable leaf.
        const WRITE     = 0x04;
        /// Marks a [`PTE`] mapping as an executable leaf, allowing the CPU to fetch instructions from this page.
        const EXECUTE   = 0x08;
        /// Marks a [`PTE`] mapping as accessible from user-mode.
        const USER      = 0x10;
        /// Marks a [`PTE`] mapping as global and should stay valid in the cache across flushes.
        const GLOBAL    = 0x20;
        /// Marks a [`PTE`] mapping as accessed (the memory contents have been fetched).
        const ACCESSED  = 0x40;
        /// Marks a [`PTE`] mapping as dirty (the memory contents have been modified).
        const DIRTY     = 0x80;

        /// Marks a [`PTE`] mapping as readable and writable.
        const RW        = 0x02 | 0x04;
        /// Marks a [`PTE`] mapping as readable and executable.
        const RX        = 0x02 | 0x08;
        /// Marks a [`PTE`] mapping as readable, writable, and executable.
        const RWX       = 0x02 | 0x04 | 0x08;

        /// Marks a [`PTE`] mapping as global readable.
        const GR        = 0x02 | 0x20;
        /// Marks a [`PTE`] mapping as global readable and writable.
        const GRW       = 0x02 | 0x04 | 0x20;
        /// Marks a [`PTE`] mapping as global readable and executable.
        const GRX       = 0x02 | 0x08 | 0x20;
        /// Marks a [`PTE`] mapping as global readable, writable, and executable.
        const GRWX      = 0x02 | 0x04 | 0x08 | 0x20;

        /// A mask that is used to compare if two [`PTE`] entries have matching access modifier and privileges.
        /// Equals to `RWXUG`.
        const COMPARE_MASK = 0x02 | 0x04 | 0x08 | 0x10 | 0x20;
    }
}

impl PTEFlags {
    /// Gets the compare mask of the field.
    ///
    /// This includes all access and privilege flags that matter in terms of comparation with other fields.
    #[inline]
    pub const fn get_mask(&self) -> Self {
        self.intersection(Self::COMPARE_MASK)
    }

    /// Returns `true` if two fields have matching access and privilege flags.
    #[inline]
    pub const fn matches(&self, other: &Self) -> bool {
        self.get_mask().bits() == other.get_mask().bits()
    }
}

/// A [`PageTable`] entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PTE(usize);

impl PTE {
    /// An empty [`PTE`].
    pub const EMPTY: Self = Self(0);

    /// Creates a [`PTE`] that points to the given physical address.
    pub fn new(page_addr: PAddr, flags: PTEFlags) -> Self {
        debug_assert!(
            page_addr.addr().is_multiple_of(PAGE_SIZE),
            "Address misaligned"
        );
        let flags = flags.bits() as usize & 0xff;
        let paddr = page_addr.ppn_all() << 10;
        Self(paddr | flags)
    }

    /// Gets the physical address pointed to by the entry.
    pub fn phys_addr(&self) -> PAddr {
        PAddr::new(((self.0 << 10) as isize >> 8) as usize & !(PAGE_SIZE - 1))
    }

    /// Gets the flags set on the entry.
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_retain((self.0 & 0xff) as u8)
    }

    /// Returns whether this entry is valid (i.e. [`PTEFlags::VALID`] is set).
    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::VALID)
    }

    /// Returns whether this entry is a leaf.
    ///
    /// A [`PTE`] is a leaf entry if all of these are true:
    /// - The entry is valid (i.e. [`PTEFlags::VALID`]).
    /// - Either [`PTEFlags::READ`], [`PTEFlags::WRITE`], or [`PTEFlags::EXECUTE`] is set.
    pub fn is_leaf(&self) -> bool {
        self.is_valid() && self.flags().intersects(PTEFlags::RWX)
    }

    /// Sets the entry as a non-leaf entry pointing to another [`PageTable`].
    ///
    /// # Safety
    /// `pa` must be the physical address of a [`PageTable`].
    pub unsafe fn set_table(&mut self, pa: PAddr) {
        self.set(pa, PTEFlags::VALID);
    }

    /// Sets the entry as a leaf entry pointing to a physical memory location.
    ///
    /// # Panics
    /// This function will panic if `pa` is not aligned to `level`.
    pub fn set_leaf(&mut self, pa: PAddr, level: PageLevel, flags: PTEFlags) {
        assert!(level.is_aligned(pa.addr()), "physical address misaligned");
        self.set(pa, flags | PTEFlags::VALID);
    }

    /// Clears this entry.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    fn set(&mut self, page_addr: PAddr, flags: PTEFlags) {
        let flags = flags.bits() as usize & 0xff;
        let paddr = page_addr.ppn_all() << 10;
        self.0 = flags | paddr;
    }
}

/// A page table.
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [PTE; PT_MAX_ENTRIES],
}

impl PageTable {
    /// Initializes a memory location with an empty [`PageTable`].
    ///
    /// # Safety
    /// `slot` must be dereferenceable to [PageTable](PageTable).
    pub unsafe fn init(slot: *mut MaybeUninit<Self>) -> *mut Self {
        if !slot.is_null() && slot.is_aligned() {
            // Safety: slot is not null and aligned
            unsafe {
                (*slot).write(Self {
                    entries: [PTE::EMPTY; 512],
                });
            }
        }
        slot.cast()
    }
}

/// Represents possible errors that might occur when mapping a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Virtual mapping fails because the allocator fails to provide a memory location for a page table.
    OutOfMemory,
    /// The virtual or physical address is not aligned to the requested level.
    Misaligned,
    /// The virtual address is already mapped to a physical address at the given level.
    AlreadyMapped(PAddr, PageLevel, PTEFlags),
}

/// Represents possible errors that might occur when unmapping a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    /// The virtual address is not mapped.
    NotMapped,
}

/// Performs a virtual address translation by walking the page tables starting from the given root table.
///
/// Returns a [`Some`] containing the physical address and the mapped page level, or [`None`] if:
/// - The virtual address is not mapped by any valid entry.
/// - The virtual address corresponds to a valid, but not a leaf entry.
/// - The virtual address is mapped, but the physical address is invalid at its level.
pub fn translate_virt(
    root_pt: *mut PageTable,
    va: VAddr,
    p2v: impl Fn(PAddr) -> VAddr,
) -> Option<(PAddr, PageLevel)> {
    let vpn = va.vpn();
    let mut table = root_pt;

    for curr_level in PageLevel::ALL.iter().rev() {
        let pte = unsafe { &mut (*table).entries[vpn[curr_level.level()]] };
        let flags = pte.flags();

        if !pte.is_valid() || (!flags.contains(PTEFlags::READ) && flags.contains(PTEFlags::WRITE)) {
            return None;
        } else if flags.intersects(PTEFlags::RX) {
            let pa = pte.phys_addr();
            let off_mask = (1usize << (12 + curr_level.level() * 9)) - 1;

            if pa.addr() & off_mask != 0 {
                return None;
            } else {
                let pa = pa + (va.addr() & off_mask);
                return Some((pa, *curr_level));
            }
        } else {
            let next_pa = pte.phys_addr();
            table = p2v(next_pa).as_mut();
        }
    }
    None
}

/// Maps a virtual address to a given physical address at a certain level, using the provided root table.
///
/// Returns an iterator of all the frame allocations made for intermediate tables, or a [`MapError`]
/// if virtual mapping fails for any reason.
pub fn map_page(
    root_pt: *mut PageTable,
    va: VAddr,
    pa: PAddr,
    level: PageLevel,
    flags: PTEFlags,
    p2v: &impl Fn(PAddr) -> VAddr,
) -> impl Iterator<Item = Result<FrameAlloc, MapError>> {
    core::iter::from_coroutine(
        #[coroutine]
        move || {
            if !level.is_aligned(va.addr()) || !level.is_aligned(pa.addr()) {
                yield Err(MapError::Misaligned);
                return;
            }

            let vpn = va.vpn();
            let mut table = root_pt;

            for curr_level in PageLevel::ALL.iter().rev() {
                let pte = unsafe { &mut (*table).entries[vpn[curr_level.level()]] };

                if *curr_level == level {
                    if pte.is_valid() {
                        let mapped_pa = pte.phys_addr().compute_phys(&va, Some(level));
                        yield Err(MapError::AlreadyMapped(mapped_pa, *curr_level, pte.flags()));
                        return;
                    }
                    pte.set_leaf(pa, level, flags);
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
                                // Safety: PageTable has the size and alignment of one physical frame
                                table = unsafe { PageTable::init(table_uninit) };
                                // Safety: pa points to the base of the table_uninit frame
                                unsafe { pte.set_table(alloc.start_addr()) };

                                yield Ok(alloc);
                            }
                            None => {
                                yield Err(MapError::OutOfMemory);
                                return;
                            }
                        }
                    } else {
                        let mapped_pa = pte.phys_addr().compute_phys(&va, Some(level));
                        yield Err(MapError::AlreadyMapped(mapped_pa, *curr_level, pte.flags()));
                        return;
                    }
                }
            }
        },
    )
}

/// Unmaps a virtual address from the address space within the provided root table.
///
/// Returns the [`PageLevel`] of the entry that was cleared, or an [`UnmapError`] if virtual
/// unmapping fails for any reason.
pub fn unmap_page(
    root_pt: *mut PageTable,
    va: VAddr,
    p2v: impl Fn(PAddr) -> VAddr,
) -> Result<PageLevel, UnmapError> {
    let vpn = va.vpn();
    let mut table = root_pt;

    for level in (0..=3).rev() {
        let pte = unsafe { &mut (*table).entries[vpn[level]] };
        if pte.is_valid() && !pte.is_leaf() {
            let next_pa = pte.phys_addr();
            table = p2v(next_pa).as_mut();
        } else if pte.is_valid() {
            pte.clear();
            return Ok(PageLevel::ALL[level]);
        } else {
            return Err(UnmapError::NotMapped);
        }
    }
    unreachable!()
}

/// Probes `satp` to determine the maximum implemented value of address space ID.
pub fn get_max_asid() -> usize {
    let mut satp = satp::read();
    let old_asid = satp.asid();
    satp.set_asid(usize::MAX);
    unsafe { satp::write(satp) };

    satp = satp::read();
    let max_asid = satp.asid();
    satp.set_asid(old_asid);
    unsafe { satp::write(satp) };

    max_asid
}

/// Switch to another address space. A [`flush_address_space`] call should follow this switch
/// if the new address space reuses a previously cached address space ID.
pub fn switch_address_space(addrsp: &AddressSpace) {
    let mut satp = satp::read();
    satp.set_ppn(addrsp.root_pa().ppn_all());
    satp.set_asid(addrsp.id());
    satp.set_mode(satp::Mode::Sv48);
    unsafe { satp::write(satp) };
}

/// Flushes the translation cache for all pages within the given address space.
pub fn flush_address_space(addrsp: &AddressSpace) {
    riscv::asm::sfence_vma(addrsp.id(), 0);
}

/// Flushes the translation cache for a page within the given address space.
pub fn flush_address_space_at(addrsp: &AddressSpace, at: VAddr) {
    riscv::asm::sfence_vma(addrsp.id(), at.addr());
}
