use core::{alloc::Layout, mem::ManuallyDrop, ptr::NonNull};

use crate::{arch::{PTEFlags, PageSize, VAddr}, mem::{self, FrameAlloc, PAGE_SIZE}, println};
use super::HeapError;

#[derive(Debug)]
struct LinkedListHeapEntry {
    used: bool,
    size: usize,
    addr: VAddr,
    next: Option<NonNull<LinkedListHeapEntry>>,
}

impl LinkedListHeapEntry {
    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn effective_size(layout: Layout) -> usize {
        size_of::<Self>() + layout.size()
    }
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
            let eff_size = LinkedListHeapEntry::effective_size(layout);
            let mut used = layout.size();
            
            if eff_size + size_of::<LinkedListHeapEntry>() <= v.size {
                let ent_next = ent.byte_add(eff_size);
                ent_next.write(LinkedListHeapEntry {
                    used: false,
                    size: v.size - eff_size - size_of::<LinkedListHeapEntry>(),
                    addr: VAddr::from_ptr(ent_next.add(1).as_ptr()),
                    next: v.next,
                });
                v.next = Some(ent_next);
                used += size_of::<LinkedListHeapEntry>();
            }
            
            self.used += eff_size + used;
            self.free -= eff_size + used;
            v.used = true;
            v.size = layout.size();
            Some((v.addr, used))
        }
    }

    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn find_entry_first_fit(&self, layout: Layout) -> Option<NonNull<LinkedListHeapEntry>> {
        if self.free < layout.size() { return None }
        let mut ent = self.head;
        
        while let Some(mut v) = ent {
            let v = unsafe { v.as_mut() };
            if v.size >= layout.size() { return ent }
            ent = v.next;
        }
        None
    }
}

#[derive(Debug)]
pub struct LinkedListHeap {
    used: usize,
    free: usize,
    next_va: VAddr,
    bump_size: usize,
    head: NonNull<LinkedListHeapRegion>,
}

impl LinkedListHeap {
    pub fn new(base_addr: VAddr, bump_size: usize) -> Result<Self, HeapError> {
        let base_addr = VAddr::new((base_addr.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1));
        let bump_size = (bump_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let alloc = ManuallyDrop::new(mem::palloc::alloc(bump_size).ok_or(HeapError::AllocationFailed)?);

        mem::vms::acquire(|mut g| -> Result<(), HeapError> {
            let mut va = base_addr;
            let mut pa = alloc.base_addr();
            let mut allocated = 0;

            while allocated < bump_size {
                let size = PageSize::select_size(va, pa, bump_size - allocated).ok_or(HeapError::MappingFailed)?;
                g.map_page(va, pa, size, PTEFlags::RW).map_err(|_| HeapError::MappingFailed)?.forget();
                va = va + size.size();
                pa = pa + size.size();
                allocated += size.size();
            }
            
            debug_assert_eq!(crate::arch::translate_virt(g.root_pt().unwrap(), base_addr, mem::vms::phys_to_virt), Some(alloc.base_addr()));
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
                next_va: base_addr + bump_size,
                bump_size,
                head: NonNull::new_unchecked(reg),
            })
        }
    }

    fn grow(&mut self) -> Result<(), HeapError> {
        let base_addr = self.next_va;
        let alloc = ManuallyDrop::new(mem::palloc::alloc(self.bump_size).ok_or(HeapError::AllocationFailed)?);

        mem::vms::acquire(|mut g| {
            let mut va = base_addr;
            let mut pa = alloc.base_addr();
            let mut allocated = 0;

            while allocated < self.bump_size {
                let size = PageSize::select_size(va, pa, self.bump_size - allocated).ok_or(HeapError::MappingFailed)?;
                g.map_page(va, pa, size, PTEFlags::RW).map_err(|_| HeapError::MappingFailed)?.forget();
                va = va + size.size();
                pa = pa + size.size();
                allocated += size.size();
            }
            
            debug_assert_eq!(crate::arch::translate_virt(g.root_pt().unwrap(), base_addr, mem::vms::phys_to_virt), Some(alloc.base_addr()));
            Ok(())
        })?;

        unsafe {
            let reg: *mut LinkedListHeapRegion = base_addr.as_mut();
            let ent: *mut LinkedListHeapEntry = reg.add(1).cast();

            let init_addr = VAddr::from_ptr(ent.add(1));
            let init_size = base_addr + self.bump_size - init_addr;
            reg.write(LinkedListHeapRegion { used: 0, free: init_size, head: Some(NonNull::new_unchecked(ent)), next: None });
            ent.write(LinkedListHeapEntry { used: false, size: init_size, addr: init_addr, next: None });

            self.next_va = self.next_va + self.bump_size;
            self.free += init_size;
            self.head.as_mut().next = Some(NonNull::new_unchecked(reg));
            Ok(())
        }
    }

    // fn find_region_include(&self, va: VAddr) -> Option<NonNull<LinkedListHeapRegion>> {
    //     todo!()
    // }

    /// # Safety
    /// `layout` must be padded to the alignment of [LinkedListHeapEntry](LinkedListHeapEntry).
    unsafe fn find_region_first_fit(&self, layout: Layout) -> Option<NonNull<LinkedListHeapRegion>> {
        if self.free < layout.size() { return None };
        
        unsafe {
            let mut reg = Some(self.head);
            while let Some(mut curr) = reg {
                let curr = curr.as_mut();
                if curr.free >= layout.size() { return reg }
                reg = curr.next;
            }
        }
        None
    }

    fn align_layout(layout: Layout) -> Option<Layout> {
        Some(layout.align_to(core::mem::align_of::<LinkedListHeapEntry>()).ok()?.pad_to_align())
    }
}

impl super::Heap for LinkedListHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match Self::align_layout(layout) {
            None => core::ptr::null_mut(),
            Some(layout) => {
                if self.free < layout.size() {
                    if let Err(_) = self.grow() {
                        return core::ptr::null_mut()
                    }
                }
                
                println!("HEAP PRE_ALLOC {:?} <- LO {:?}", self, layout);
                let mut reg = unsafe { self.find_region_first_fit(layout) };
                match reg.as_mut() {
                    None => core::ptr::null_mut(),
                    Some(reg) =>
                        match unsafe { reg.as_mut().alloc(layout) } {
                            None => core::ptr::null_mut(),
                            Some((va, eff_size)) => {
                                self.used += eff_size;
                                self.free -= eff_size;
                                println!("HEAP POST_ALLOC {:?} -> VA {:?}", self, va);
                                va.as_mut()
                            }
                        },
                }
            },
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
