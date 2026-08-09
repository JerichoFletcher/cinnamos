use core::fmt::Debug;

use alloc::vec::Vec;
use spin::Mutex;

use crate::{
    arch::{
        self, MapError, PAGE_TABLE_DEPTH, PAddr, PTEFlags, PageLevel, PageTable, UnmapError, VAddr,
    },
    mem::{PhysFrameAlloc, physalloc::FrameAlloc, virt::VirtAlloc, vmalloc::PageAlloc},
};

/// Represents possible errors that may arise from [`AddressSpace`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    /// A mapping is attempted between a virtual and physical page of mismatched sizes.
    MismatchedSize,
    /// The virtual or physical address is not aligned to page boundaries.
    AddressMisaligned(VAddr, PAddr),
    /// A virtual address is already mapped to a different physical address.
    MappedDifferentAddress { expected: PAddr, actual: PAddr },
    /// The virtual address is already mapped to the expected physical address,
    /// but under a different set of page flags.
    MappedDifferentFlags {
        expected: PTEFlags,
        actual: PTEFlags,
    },
    /// An error happened during virtual mapping. The inner error is propagated.
    Map(MapError),
    /// An error happened during virtual unmapping. The inner error is propagated.
    Unmap(UnmapError),
}

/// An [`AddressSpace`] holds the page tables for an address space and provides methods
/// to modify the mappings within.
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
    /// Creates a possibly initialized address space.
    ///
    /// # Safety
    /// - `root_ptr` must point to the virtual page mapped to `root` and stay mapped for the entire lifetime of the [AddressSpace].
    /// - `p2v` must translate physical page table addresses to valid, mapped virtual addresses.
    pub unsafe fn new(
        id: usize,
        root_ptr: *mut PageTable,
        root: FrameAlloc,
        init_tables_cap: usize,
        p2v: &'a dyn Fn(PAddr) -> VAddr,
    ) -> Result<Self, AddressSpaceError> {
        let addrsp = Self {
            id,
            root_ptr,
            p2v,
            root,
            tables: Mutex::new(Vec::with_capacity(init_tables_cap)),
        };
        addrsp.map_raw_relaxed(
            p2v(addrsp.root_pa()),
            addrsp.root_pa(),
            addrsp.root.size(),
            PTEFlags::GRW,
        )?;
        Ok(addrsp)
    }

    /// Gets the ID of this address space.
    #[inline]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Switches the address translator and remaps owned page tables to the new space.
    ///
    /// # Safety
    /// The new `p2v` must translate physical page table addresses to valid, mapped virtual addresses.
    pub unsafe fn remap(
        &mut self,
        p2v: &'a dyn Fn(PAddr) -> VAddr,
    ) -> Result<(), AddressSpaceError> {
        self.map_raw_relaxed(
            p2v(self.root_pa()),
            self.root_pa(),
            self.root.size(),
            PTEFlags::GRW,
        )?;
        for t in self.tables.lock().iter() {
            self.map_raw_relaxed(p2v(t.start_addr()), t.start_addr(), t.size(), PTEFlags::GRW)?;
        }
        self.root_ptr = p2v(self.root_pa()).as_mut();
        self.p2v = p2v;
        Ok(())
    }

    /// Reallocates owned buffers.
    /// Useful when heap allocation uses a different virtual space than the one the current heap
    /// allocations point to.
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

    /// The physical address of the root page table.
    pub fn root_pa(&self) -> PAddr {
        self.root.start_addr()
    }

    /// Maps a virtual page to a physical page within this address space.
    pub fn map(
        &self,
        virt: &PageAlloc,
        phys: &FrameAlloc,
        flags: PTEFlags,
    ) -> Result<(), AddressSpaceError> {
        if virt.page_count() != phys.frame_count() {
            return Err(AddressSpaceError::MismatchedSize);
        }
        self.map_raw_relaxed(virt.start_addr(), phys.start_addr(), phys.size(), flags)?;
        Ok(())
    }

    /// Unmaps a virtual page within this address space.
    pub fn unmap(&self, virt: &PageAlloc) -> Result<(), AddressSpaceError> {
        self.unmap_raw(virt.start_addr(), virt.size())
    }

    /// Maps a virtual region to a physical region of the same size within this address space.
    /// The mapping is done with a relaxed policy: if any subset of the region is already mapped
    /// to the same physical address, with the same [`PTEFlags`], they are skipped.
    pub fn map_raw_relaxed(
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
            let mut next_lv = PageLevel::select_size(va, pa, pa_end - pa)
                .ok_or(AddressSpaceError::AddressMisaligned(va, pa))?;

            match arch::map_page(self.root_ptr, va, pa, next_lv, flags, &self.p2v).try_fold(
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
                        self.map_raw_relaxed(
                            (self.p2v)(a.start_addr()),
                            a.start_addr(),
                            a.size(),
                            PTEFlags::GRW,
                        )?;
                        self.tables.lock().push(a);
                    }
                }
                Err(e) => match e {
                    MapError::AlreadyMapped(mapped_pa, mapped_lv, mapped_flags) => {
                        // Correctness test
                        debug_assert!(next_lv <= mapped_lv);

                        if pa != mapped_pa {
                            return Err(AddressSpaceError::MappedDifferentAddress {
                                expected: pa,
                                actual: mapped_pa,
                            });
                        } else if !flags.matches(&mapped_flags) {
                            return Err(AddressSpaceError::MappedDifferentFlags {
                                expected: flags.get_mask(),
                                actual: mapped_flags.get_mask(),
                            });
                        } else {
                            next_lv = mapped_lv;
                        }
                    }
                    _ => return Err(AddressSpaceError::Map(e)),
                },
            };

            va = va + next_lv.size();
            pa = pa + next_lv.size();
        }
        Ok(())
    }

    /// Unmaps a virtual region within this address space.
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

    #[inline]
    pub fn translate_virt(&self, va: VAddr) -> Option<(PAddr, PageLevel)> {
        arch::translate_virt(self.root_ptr, va, self.p2v)
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
