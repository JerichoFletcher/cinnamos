use core::fmt::Debug;

use alloc::vec::Vec;
use spin::Mutex;

use crate::{
    arch::{
        self, MapError, PAGE_TABLE_DEPTH, PAddr, PTEFlags, PageLevel, PageTable, UnmapError, VAddr,
    },
    mem::{PhysFrameAlloc, physalloc::FrameAlloc, virt::VirtAlloc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    AddressMisaligned(VAddr, PAddr),
    Map(MapError),
    Unmap(UnmapError),
}

pub struct AddressSpace<'a> {
    /// The ID of this address space. Used to derive the ASID for virtual paging protocols.
    id: usize,
    /// Points to the root page table. Must always point to the physical page at [`root`](AddressSpace::root).
    root_ptr: *mut PageTable,
    /// The physical-to-virtual address translation function used to walk page tables within this address space.
    p2v: &'a dyn Fn(PAddr) -> VAddr,
    /// The physical page allocation for the frame containing the root page table.
    root: FrameAlloc,
    /// Allocations of all the intermediate tables associated with this address space.
    tables: Mutex<Vec<FrameAlloc>>,
}

impl<'a> AddressSpace<'a> {
    /// # Safety
    /// - `root_ptr` must point to the virtual page mapped to `root` and stay mapped for the entire lifetime of the [AddressSpace].
    /// - `p2v` must translate physical page table addresses to valid, mapped virtual addresses.
    pub unsafe fn new(
        id: usize,
        root_ptr: *mut PageTable,
        root: FrameAlloc,
        tables: Vec<FrameAlloc>,
        p2v: &'a dyn Fn(PAddr) -> VAddr,
    ) -> Result<Self, AddressSpaceError> {
        let addrsp = Self {
            id,
            root_ptr,
            p2v,
            root,
            tables: Mutex::new(tables),
        };
        addrsp.map_raw_skip_mapped(
            p2v(addrsp.root_pa()),
            addrsp.root_pa(),
            addrsp.root.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        )?;
        Ok(addrsp)
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    /// # Safety
    /// The new `p2v` must translate physical page table addresses to valid, mapped virtual addresses.
    pub unsafe fn remap(
        &mut self,
        p2v: &'a dyn Fn(PAddr) -> VAddr,
    ) -> Result<(), AddressSpaceError> {
        self.map_raw_skip_mapped(
            p2v(self.root_pa()),
            self.root_pa(),
            self.root.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        )?;
        for t in self.tables.lock().iter() {
            self.map_raw_skip_mapped(
                p2v(t.start_addr()),
                t.start_addr(),
                t.size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )?;
        }
        self.root_ptr = p2v(self.root_pa()).as_mut();
        self.p2v = p2v;
        Ok(())
    }

    pub fn realloc(&self) {
        let tables = self.tables.lock();
        let tables_cap = tables.len();
        drop(tables);

        let mut new_tables = Vec::with_capacity(tables_cap);
        let mut old_tables = self.tables.lock();
        while let Some(t) = old_tables.pop() {
            new_tables.push(t);
        }
        *old_tables = new_tables;
    }

    pub fn root_pa(&self) -> PAddr {
        self.root.start_addr()
    }

    pub fn map(
        &self,
        virt: &impl VirtAlloc,
        phys: &FrameAlloc,
        flags: PTEFlags,
    ) -> Result<(), AddressSpaceError> {
        assert_eq!(
            virt.size(),
            phys.size(),
            "Attempted to map unequal physical and virtual regions"
        );
        self.map_raw(virt.start_addr(), phys.start_addr(), phys.size(), flags)?;
        Ok(())
    }

    pub fn unmap(&self, virt: &impl VirtAlloc) -> Result<(), AddressSpaceError> {
        self.unmap_raw(virt.start_addr(), virt.size())
    }

    pub fn map_raw(
        &self,
        va: VAddr,
        pa: PAddr,
        size_bytes: usize,
        flags: PTEFlags,
    ) -> Result<(), AddressSpaceError> {
        log::trace!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} size={} map id={}",
            &va,
            va + size_bytes,
            &pa,
            pa + size_bytes,
            size_bytes,
            self.id,
        );
        let mut va = va;
        let mut pa = pa;
        let pa_end = pa + size_bytes;

        while pa < pa_end {
            let next_size = PageLevel::select_size(va, pa, pa_end - pa)
                .ok_or(AddressSpaceError::AddressMisaligned(va, pa))?;

            let (_, allocs) = arch::map_page(self.root_ptr, va, pa, next_size, flags, &self.p2v)
                .try_fold(
                    (0, [const { None }; PAGE_TABLE_DEPTH]),
                    |(i, mut allocs), v| match v {
                        Ok(a) => {
                            allocs[i] = Some(a);
                            Ok((i + 1, allocs))
                        }
                        Err(e) => Err(e),
                    },
                )
                .map_err(AddressSpaceError::Map)?;
            for a in allocs.into_iter().flatten() {
                self.map_raw_skip_mapped(
                    (self.p2v)(a.start_addr()),
                    a.start_addr(),
                    a.size(),
                    PTEFlags::GLOBAL | PTEFlags::RW,
                )?;
                self.tables.lock().push(a);
            }

            va = va + next_size.size();
            pa = pa + next_size.size();
        }
        Ok(())
    }

    pub fn map_raw_skip_mapped(
        &self,
        va: VAddr,
        pa: PAddr,
        size_bytes: usize,
        flags: PTEFlags,
    ) -> Result<(), AddressSpaceError> {
        log::trace!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} size={} map id={}",
            &va,
            va + size_bytes,
            &pa,
            pa + size_bytes,
            size_bytes,
            self.id,
        );
        let mut va = va;
        let mut pa = pa;
        let pa_end = pa + size_bytes;

        while pa < pa_end {
            let mut next_size = PageLevel::select_size(va, pa, pa_end - pa)
                .ok_or(AddressSpaceError::AddressMisaligned(va, pa))?;

            match arch::map_page(self.root_ptr, va, pa, next_size, flags, &self.p2v).try_fold(
                (0, [const { None }; PAGE_TABLE_DEPTH]),
                |(i, mut allocs), v| match v {
                    Ok(a) => {
                        allocs[i] = Some(a);
                        Ok((i + 1, allocs))
                    }
                    Err(e) => Err(e),
                },
            ) {
                Ok((_, allocs)) => {
                    for a in allocs.into_iter().flatten() {
                        self.map_raw_skip_mapped(
                            (self.p2v)(a.start_addr()),
                            a.start_addr(),
                            a.size(),
                            PTEFlags::GLOBAL | PTEFlags::RW,
                        )?;
                        self.tables.lock().push(a);
                    }
                }
                Err(e) => match e {
                    MapError::AlreadyMapped(_, mapped_pa, mapped_level) => {
                        log::trace!(
                            "{:#016x} -> {:#016x} size={} map id={} already mapped level={:?}",
                            va,
                            mapped_pa,
                            next_size.size(),
                            self.id,
                            mapped_level,
                        );
                        next_size = mapped_level;
                    }
                    _ => return Err(AddressSpaceError::Map(e)),
                },
            };

            va = va + next_size.size();
            pa = pa + next_size.size();
        }
        Ok(())
    }

    pub fn unmap_raw(&self, va: VAddr, size_bytes: usize) -> Result<(), AddressSpaceError> {
        log::trace!(
            "{:#016x} .. {:#016x} size={} unmap id={}",
            &va,
            va + size_bytes,
            size_bytes,
            self.id,
        );
        let mut va = va;
        let va_end = va + size_bytes;

        while va < va_end {
            let next_size =
                arch::unmap_page(self.root_ptr, va, self.p2v).map_err(AddressSpaceError::Unmap)?;
            va = va + next_size.size();
        }
        Ok(())
    }
}

// Safety:
// - root_ptr is always mapped to the frames corresponding to root
// - root_ptr is never exposed outside of the struct, which prevents it from outliving the struct
unsafe impl Send for AddressSpace<'_> {}
// Safety: Mutations via &self are synchronized across harts
unsafe impl Sync for AddressSpace<'_> {}

impl Debug for AddressSpace<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddressSpace")
            .field("root_ptr", &self.root_ptr)
            .field("root", &self.root)
            .finish()
    }
}
