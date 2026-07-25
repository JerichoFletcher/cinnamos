use core::marker::PhantomData;

use alloc::{collections::linked_list::LinkedList, vec::Vec};

use crate::{
    arch::VAddr,
    mem::{self, PhysFrameAlloc, physalloc::Alloc},
};

pub struct Slab<T> {
    free: usize,
    total: usize,
    bitmap: Vec<u64>,
    base: VAddr,
    _alloc: Alloc,
    _marker: PhantomData<T>,
}

impl<T> Slab<T> {
    pub fn new(frame_count: usize) -> Option<Self> {
        let alloc = mem::physalloc::alloc(frame_count)?;
        let total = alloc.size() / size_of::<T>();

        let bitmap = alloc::vec![0; total.max(64) / 64];
        let base = mem::vms::phys_to_virt(alloc.start_addr());

        Some(Self {
            free: 0,
            total,
            bitmap,
            base,
            _alloc: alloc,
            _marker: PhantomData,
        })
    }

    pub fn alloc(&mut self) -> *mut T {
        if let Some((bitmap_index, bits)) = self
            .bitmap
            .iter_mut()
            .enumerate()
            .filter(|(_, bits)| **bits != u64::MAX)
            .next()
        {
            for bit_index in 0..u64::BITS as usize {
                let mask = 1 << bit_index;
                if *bits & mask == 0 {
                    *bits |= mask;
                    let index = bitmap_index * u64::BITS as usize + bit_index;
                    return self.index_to_va(index).as_mut();
                }
            }
        }
        core::ptr::null_mut()
    }

    pub fn dealloc(&mut self, ptr: *mut T) {
        let index = self.va_to_index(VAddr::from_ptr(ptr));
        let bitmap_index = index / u64::BITS as usize;
        let bit_index = index % u64::BITS as usize;

        let bits = &mut self.bitmap[bitmap_index];
        *bits &= !(1 << bit_index);
    }

    pub fn contains(&self, ptr: *const T) -> bool {
        self.base.addr() <= (ptr as usize)
            && (ptr as usize) < self.base.addr() + self.total * size_of::<T>()
    }

    fn index_to_va(&self, index: usize) -> VAddr {
        self.base + index * size_of::<T>()
    }

    fn va_to_index(&self, va: VAddr) -> usize {
        (va - self.base) / size_of::<T>()
    }
}

pub struct SlabAllocator<T: 'static> {
    slab_frame_count: usize,
    init_fn: Option<&'static dyn Fn(*mut T) -> Result<(), ()>>,
    deinit_fn: Option<&'static dyn Fn(*mut T)>,
    slabs: LinkedList<Slab<T>>,
    _marker: PhantomData<T>,
}

impl<T> SlabAllocator<T> {
    pub const fn new(
        slab_frame_count: usize,
        init_fn: Option<&'static dyn Fn(*mut T) -> Result<(), ()>>,
        deinit_fn: Option<&'static dyn Fn(*mut T)>,
    ) -> Self {
        Self {
            slab_frame_count,
            init_fn,
            deinit_fn,
            slabs: LinkedList::new(),
            _marker: PhantomData,
        }
    }

    pub fn alloc(&mut self) -> *mut T {
        for slab in self.slabs.iter_mut() {
            if slab.free > 0 {
                let ptr = slab.alloc();
                return match &self.init_fn {
                    Some(init) => match init(ptr) {
                        Ok(()) => ptr,
                        Err(()) => core::ptr::null_mut(),
                    },
                    None => core::ptr::null_mut(),
                };
            }
        }
        match Slab::<T>::new(self.slab_frame_count) {
            Some(mut slab) => {
                let ptr = slab.alloc();
                self.slabs.push_front(slab);

                match &self.init_fn {
                    Some(init) => match init(ptr) {
                        Ok(()) => ptr,
                        Err(()) => core::ptr::null_mut(),
                    },
                    None => core::ptr::null_mut(),
                }
            }
            None => core::ptr::null_mut(),
        }
    }

    pub fn dealloc(&mut self, ptr: *mut T) {
        for slab in self.slabs.iter_mut() {
            if slab.contains(ptr) {
                if let Some(deinit) = &self.deinit_fn {
                    deinit(ptr);
                }
                slab.dealloc(ptr);
                return;
            }
        }
    }
}
