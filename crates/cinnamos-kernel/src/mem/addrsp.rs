use core::fmt::Debug;

use alloc::vec::Vec;
use spin::Mutex;

use crate::{
    arch::{
        self, MapError, PAGE_TABLE_DEPTH, PAddr, PTEFlags, PageSize, PageTable, UnmapError, VAddr,
    },
    mem::{PhysFrameAlloc, physalloc::Alloc, virt::VirtAlloc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    AddressMisaligned(VAddr, PAddr),
    Map(MapError),
    Unmap(UnmapError),
}

struct AddressSpaceBuffers<T: VirtAlloc> {
    tables: Vec<Alloc>,
    virtallocs: Vec<T>,
}

impl<T: VirtAlloc> AddressSpaceBuffers<T> {
    fn realloc(&mut self, mut tables: Vec<Alloc>, mut virtallocs: Vec<T>) {
        tables.clear();
        virtallocs.clear();

        while let Some(t) = self.tables.pop() {
            tables.push(t);
        }
        while let Some(v) = self.virtallocs.pop() {
            virtallocs.push(v);
        }

        self.tables = tables;
        self.virtallocs = virtallocs;
    }
}

pub struct AddressSpace<'a, T: VirtAlloc> {
    root_ptr: *mut PageTable,
    p2v: &'a dyn Fn(PAddr) -> VAddr,
    root: Alloc,
    buffers: Mutex<AddressSpaceBuffers<T>>,
}

impl<'a, T: VirtAlloc> AddressSpace<'a, T> {
    pub fn new(
        root_ptr: *mut PageTable,
        root: Alloc,
        tables: Vec<Alloc>,
        p2v: &'a dyn Fn(PAddr) -> VAddr,
    ) -> Result<Self, AddressSpaceError> {
        let addrsp = Self {
            root_ptr,
            p2v,
            root,
            buffers: Mutex::new(AddressSpaceBuffers {
                tables,
                virtallocs: alloc::vec![],
            }),
        };
        addrsp.map_raw(
            p2v(addrsp.root_pa()),
            addrsp.root_pa(),
            addrsp.root.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        )?;
        Ok(addrsp)
    }

    pub fn remap(&mut self, p2v: &'a dyn Fn(PAddr) -> VAddr) -> Result<(), AddressSpaceError> {
        self.map_raw_skip_mapped(
            p2v(self.root_pa()),
            self.root_pa(),
            self.root.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        )?;
        for t in self.buffers.lock().tables.iter() {
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
        let g = self.buffers.lock();
        let tables_cap = g.tables.len();
        let virtallocs_cap = g.virtallocs.len();
        drop(g);

        let tables = Vec::with_capacity(tables_cap);
        let virtallocs = Vec::with_capacity(virtallocs_cap);
        self.buffers.lock().realloc(tables, virtallocs);
    }

    pub const fn root_ptr(&self) -> *mut PageTable {
        self.root_ptr
    }

    pub fn root_pa(&self) -> PAddr {
        self.root.start_addr()
    }

    pub fn map(&self, virt: T, phys: &Alloc, flags: PTEFlags) -> Result<(), AddressSpaceError> {
        assert_eq!(
            virt.size(),
            phys.size(),
            "Attempted to map unequal physical and virtual regions"
        );
        self.map_raw(virt.start_addr(), phys.start_addr(), phys.size(), flags)?;
        self.buffers.lock().virtallocs.push(virt);
        Ok(())
    }

    pub fn unmap(&self, virt: &T) -> Result<(), AddressSpaceError> {
        self.unmap_raw(virt.start_addr(), virt.size())
    }

    pub fn map_raw(
        &self,
        va: VAddr,
        pa: PAddr,
        size_bytes: usize,
        flags: PTEFlags,
    ) -> Result<(), AddressSpaceError> {
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
                self.buffers.lock().tables.push(a);
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
                        self.buffers.lock().tables.push(a);
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

impl<T: VirtAlloc> Drop for AddressSpace<'_, T> {
    fn drop(&mut self) {
        let mut data = self.buffers.lock();
        while let Some(v) = data.virtallocs.pop() {
            let _ = self.unmap(&v);
        }
    }
}

impl<T: VirtAlloc> Debug for AddressSpace<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddressSpace")
            .field("root_ptr", &self.root_ptr)
            .field("root", &self.root)
            .finish()
    }
}
