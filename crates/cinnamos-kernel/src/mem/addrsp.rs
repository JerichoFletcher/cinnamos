use core::fmt::Debug;

use alloc::vec::Vec;
use spin::Mutex;

use crate::{
    arch::{
        self, MapError, PAGE_TABLE_DEPTH, PAddr, PTEFlags, PageSize, PageTable, UnmapError, VAddr,
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
    id: usize,
    root_ptr: *mut PageTable,
    p2v: &'a dyn Fn(PAddr) -> VAddr,
    root: FrameAlloc,
    tables: Mutex<Vec<FrameAlloc>>,
}

impl<'a> AddressSpace<'a> {
    pub fn new(
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
        addrsp.map_raw(
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

    pub fn remap(&mut self, p2v: &'a dyn Fn(PAddr) -> VAddr) -> Result<(), AddressSpaceError> {
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

    pub const fn root_ptr(&self) -> *mut PageTable {
        self.root_ptr
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
            "map 0x{:016x} .. 0x{:016x} -> 0x{:016x} .. 0x{:016x} size={}",
            &va, va + size_bytes,
            &pa, pa + size_bytes,
            size_bytes,
        );
        let mut va = va;
        let mut pa = pa;
        let pa_end = pa + size_bytes;

        while pa < pa_end {
            let next_size = PageSize::select_size(va, pa, pa_end - pa)
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
            "map 0x{:016x} .. 0x{:016x} -> 0x{:016x} .. 0x{:016x} size={}",
            &va, va + size_bytes,
            &pa, pa + size_bytes,
            size_bytes,
        );
        let mut va = va;
        let mut pa = pa;
        let pa_end = pa + size_bytes;

        while pa < pa_end {
            let next_size = PageSize::select_size(va, pa, pa_end - pa)
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
                    MapError::AlreadyMapped(_, _) => (),
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
            "unmap 0x{:016x} .. 0x{:016x} size={}",
            &va, va + size_bytes,
            size_bytes,
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

impl Debug for AddressSpace<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddressSpace")
            .field("root_ptr", &self.root_ptr)
            .field("root", &self.root)
            .finish()
    }
}
