use core::{alloc::Layout, mem::ManuallyDrop, ptr::NonNull};

use super::HeapError;
use crate::{
    arch::{PTEFlags, PageSize, VAddr},
    mem::{self, PAGE_SIZE, PhysFrameAlloc},
};

#[derive(Debug)]
struct LinkedListHeapEntry {
    used: bool,
    size: usize,
    addr: VAddr,
    next: Option<NonNull<LinkedListHeapEntry>>,
}

#[derive(Debug)]
struct LinkedListHeapRegion {
    used: usize,
    free: usize,
    head: Option<NonNull<LinkedListHeapEntry>>,
    next: Option<NonNull<LinkedListHeapRegion>>,
}

impl LinkedListHeapRegion {
    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn alloc(&mut self, layout: Layout) -> Option<(VAddr, usize)> {
        let mut ent = unsafe { self.find_entry_first_fit(layout)? };

        unsafe {
            let v = ent.as_mut();
            let mut used = layout.size();

            if used + size_of::<LinkedListHeapEntry>() <= v.size {
                used += size_of::<LinkedListHeapEntry>();
                let ent_next = ent.byte_add(size_of::<LinkedListHeapEntry>() + layout.size());
                ent_next.write(LinkedListHeapEntry {
                    used: false,
                    size: v.size - used,
                    addr: VAddr::from_ptr(ent_next.add(1).as_ptr()),
                    next: v.next,
                });
                v.next = Some(ent_next);
            }

            self.used += used;
            self.free -= used;
            v.used = true;
            v.size = layout.size();
            Some((v.addr, used))
        }
    }

    /// # Safety
    /// - `va` has to correspond to the [LinkedListHeapEntry::addr](LinkedListHeapEntry::addr) of one of the entries within this region.
    /// - `layout` has to be the same layout used to allocate the above entry.
    unsafe fn dealloc(&mut self, va: VAddr, layout: Layout) -> usize {
        if let Some(mut ent) = self.find_entry(va) {
            unsafe {
                let v = ent.as_mut();
                if v.size == layout.size() {
                    let mut freed = layout.size();
                    v.used = false;

                    let mut neighbor_ptr = v.next;
                    while let Some(mut neighbor) = neighbor_ptr {
                        let neighbor = neighbor.as_mut();
                        if neighbor.used {
                            break;
                        }

                        freed += size_of::<LinkedListHeapEntry>();
                        v.size += size_of::<LinkedListHeapEntry>() + neighbor.size;
                        v.next = neighbor.next;
                        neighbor_ptr = v.next;
                    }

                    self.used -= freed;
                    self.free += freed;
                    return freed;
                }
            }
        }
        0
    }

    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn find_entry_first_fit(&self, layout: Layout) -> Option<NonNull<LinkedListHeapEntry>> {
        if self.free < layout.size() {
            return None;
        }
        let mut ent = self.head;

        while let Some(mut v) = ent {
            let v = unsafe { v.as_mut() };
            if !v.used && v.size >= layout.size() {
                return ent;
            }
            ent = v.next;
        }
        None
    }

    fn find_entry(&self, va: VAddr) -> Option<NonNull<LinkedListHeapEntry>> {
        let mut ent = self.head;

        while let Some(mut v) = ent {
            let v = unsafe { v.as_mut() };
            if v.addr == va {
                return ent;
            }
            ent = v.next;
        }
        None
    }
}

#[derive(Debug)]
pub struct LinkedListHeap {
    used: usize,
    free: usize,
    init_va: VAddr,
    next_va: VAddr,
    bump_size: usize,
    head: NonNull<LinkedListHeapRegion>,
}

impl LinkedListHeap {
    pub fn new(base_addr: VAddr, bump_size: usize) -> Result<Self, HeapError> {
        let base_addr = VAddr::new((base_addr.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1));
        let bump_size = (bump_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let alloc =
            ManuallyDrop::new(mem::palloc::alloc(bump_size).ok_or(HeapError::AllocationFailed)?);

        mem::vms::acquire(|mut g| -> Result<(), HeapError> {
            let mut va = base_addr;
            let mut pa = alloc.base_addr();
            let mut allocated = 0;

            while allocated < bump_size {
                let size = PageSize::select_size(va, pa, bump_size - allocated)
                    .ok_or(HeapError::MappingFailed)?;
                g.map_page(va, pa, size, PTEFlags::RW)
                    .map_err(|_| HeapError::MappingFailed)?
                    .forget();
                va = va + size.size();
                pa = pa + size.size();
                allocated += size.size();
            }

            #[cfg(debug_assertions)]
            {
                debug_assert_eq!(
                    crate::arch::translate_virt(
                        g.root_pt().unwrap(),
                        base_addr,
                        mem::vms::phys_to_virt
                    ),
                    Some(alloc.base_addr())
                );
            }
            Ok(())
        })?;

        unsafe {
            let reg: *mut LinkedListHeapRegion = base_addr.as_mut();
            let ent: *mut LinkedListHeapEntry = reg.add(1).cast();

            let init_addr = VAddr::from_ptr(ent.add(1));
            let init_size = base_addr + bump_size - init_addr;
            reg.write(LinkedListHeapRegion {
                used: 0,
                free: init_size,
                head: Some(NonNull::new_unchecked(ent)),
                next: None,
            });
            ent.write(LinkedListHeapEntry {
                used: false,
                size: init_size,
                addr: init_addr,
                next: None,
            });

            Ok(Self {
                used: 0,
                free: init_size,
                init_va: base_addr,
                next_va: base_addr + bump_size,
                bump_size,
                head: NonNull::new_unchecked(reg),
            })
        }
    }

    fn grow(&mut self) -> Result<(), HeapError> {
        let base_addr = self.next_va;
        let alloc = ManuallyDrop::new(
            mem::palloc::alloc(self.bump_size).ok_or(HeapError::AllocationFailed)?,
        );

        mem::vms::acquire(|mut g| {
            let mut va = base_addr;
            let mut pa = alloc.base_addr();
            let mut allocated = 0;

            while allocated < self.bump_size {
                let size = PageSize::select_size(va, pa, self.bump_size - allocated)
                    .ok_or(HeapError::MappingFailed)?;
                g.map_page(va, pa, size, PTEFlags::RW)
                    .map_err(|_| HeapError::MappingFailed)?
                    .forget();
                va = va + size.size();
                pa = pa + size.size();
                allocated += size.size();
            }

            #[cfg(debug_assertions)]
            {
                debug_assert_eq!(
                    crate::arch::translate_virt(
                        g.root_pt().unwrap(),
                        base_addr,
                        mem::vms::phys_to_virt
                    ),
                    Some(alloc.base_addr())
                );
            }
            Ok(())
        })?;

        unsafe {
            let reg: *mut LinkedListHeapRegion = base_addr.as_mut();
            let ent: *mut LinkedListHeapEntry = reg.add(1).cast();

            let init_addr = VAddr::from_ptr(ent.add(1));
            let init_size = base_addr + self.bump_size - init_addr;
            reg.write(LinkedListHeapRegion {
                used: 0,
                free: init_size,
                head: Some(NonNull::new_unchecked(ent)),
                next: None,
            });
            ent.write(LinkedListHeapEntry {
                used: false,
                size: init_size,
                addr: init_addr,
                next: None,
            });

            self.next_va = self.next_va + self.bump_size;
            self.free += init_size;
            self.head.as_mut().next = Some(NonNull::new_unchecked(reg));
            Ok(())
        }
    }

    fn find_region_include(&self, va: VAddr) -> Option<NonNull<LinkedListHeapRegion>> {
        if va < self.init_va {
            return None;
        }

        unsafe {
            let mut reg = Some(self.head);
            let mut va_hi = self.init_va + self.bump_size;
            while let Some(curr) = reg {
                let curr = curr.as_ref();
                if va < va_hi {
                    return reg;
                }
                reg = curr.next;
                va_hi = va_hi + self.bump_size;
            }
        }
        None
    }

    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn find_region_first_fit(
        &self,
        layout: Layout,
    ) -> Option<NonNull<LinkedListHeapRegion>> {
        if self.free < layout.size() {
            return None;
        };

        unsafe {
            let mut reg = Some(self.head);
            while let Some(mut curr) = reg {
                let curr = curr.as_mut();
                if curr.free >= layout.size() {
                    return reg;
                }
                reg = curr.next;
            }
        }
        None
    }

    fn align_layout(layout: Layout) -> Option<Layout> {
        Some(
            layout
                .align_to(core::mem::align_of::<LinkedListHeapEntry>())
                .ok()?
                .pad_to_align(),
        )
    }
}

impl super::Heap for LinkedListHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match Self::align_layout(layout) {
            None => core::ptr::null_mut(),
            Some(layout) => {
                if self.free < layout.size() {
                    if let Err(_) = self.grow() {
                        return core::ptr::null_mut();
                    }
                }

                unsafe {
                    let mut reg = self.find_region_first_fit(layout);
                    match reg.as_mut() {
                        None => core::ptr::null_mut(),
                        Some(reg) => match reg.as_mut().alloc(layout) {
                            None => core::ptr::null_mut(),
                            Some((va, size_used)) => {
                                self.used += size_used;
                                self.free -= size_used;
                                va.as_mut()
                            }
                        },
                    }
                }
            }
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if let Some(layout) = Self::align_layout(layout) {
            let va = VAddr::from_ptr(ptr);
            if let Some(mut reg) = self.find_region_include(va) {
                unsafe {
                    let reg = reg.as_mut();
                    let size_freed = reg.dealloc(va, layout);
                    self.used -= size_freed;
                    self.free += size_freed;
                }
            }
        }
    }
}
