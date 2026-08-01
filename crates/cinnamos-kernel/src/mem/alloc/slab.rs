use core::{
    fmt::Debug, marker::PhantomData, ops::{Deref, DerefMut}, ptr::NonNull,
};

use alloc::{boxed::Box, collections::linked_list::LinkedList, vec::Vec};
use spin::{Mutex, RwLock};

use crate::{
    arch::VAddr,
    mem::{self, PhysFrameAlloc, physalloc::Alloc},
};

pub trait SlabInit: Sized {
    fn init() -> Option<Self>;
}

pub struct SlabBox<T> {
    ptr: NonNull<T>,
    slab: NonNull<Slab<T>>,
}

impl<T> SlabBox<T> {
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T> AsRef<T> for SlabBox<T> {
    fn as_ref(&self) -> &T {
        // Safety: ptr is valid within its slab
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> AsMut<T> for SlabBox<T> {
    fn as_mut(&mut self) -> &mut T {
        // Safety: ptr is valid within its slab
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Deref for SlabBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for SlabBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Drop for SlabBox<T> {
    fn drop(&mut self) {
        // Safety: Slabs are never dropped after creation
        unsafe {
            self.slab.as_ref().dealloc(self);
        }
    }
}

impl<T: Debug> Debug for SlabBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SlabBox")
            .field("ptr", &self.as_ptr())
            .field("value", self.as_ref())
            .finish()
    }
}

struct SlabData {
    free: usize,
    bitmap: Vec<u64>,
}

pub struct Slab<T> {
    total: usize,
    data: Mutex<SlabData>,
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
            total,
            data: Mutex::new(SlabData {
                free: total,
                bitmap,
            }),
            base,
            _alloc: alloc,
            _marker: PhantomData,
        })
    }

    pub fn alloc(&self) -> Option<SlabBox<T>> {
        let mut data = self.data.lock();

        if data.free > 0
            && let Some((bitmap_index, bits)) = data
                .bitmap
                .iter_mut()
                .enumerate()
                .find(|(_, bits)| **bits != u64::MAX)
        {
            let bit_index = (!*bits).trailing_zeros();
            *bits |= 1 << bit_index;
            let index = bitmap_index * u64::BITS as usize + bit_index as usize;
            return self.index_to_va(index).as_nonnull().map(|ptr| {
                data.free -= 1;
                SlabBox { ptr, slab: NonNull::from_ref(self) }
            });
        }
        None
    }

    /// # Safety
    /// `handle` must be a box allocated from this slab.
    pub unsafe fn dealloc(&self, handle: &mut SlabBox<T>) {
        // Safety: The pointer held by the box comes from this slab
        unsafe { handle.ptr.drop_in_place(); }

        let index = self.va_to_index(VAddr::new(handle.ptr.addr().get()));
        let bitmap_index = index / u64::BITS as usize;
        let bit_index = index % u64::BITS as usize;

        let mut data = self.data.lock();
        let bits = &mut data.bitmap[bitmap_index];
        *bits &= !(1 << bit_index);
        data.free += 1;
    }

    pub fn contains(&self, handle: &SlabBox<T>) -> bool {
        let addr = handle.as_ptr() as usize;
        self.base.addr() <= addr && addr < self.base.addr() + self.total * size_of::<T>()
    }

    fn index_to_va(&self, index: usize) -> VAddr {
        self.base + index * size_of::<T>()
    }

    fn va_to_index(&self, va: VAddr) -> usize {
        (va - self.base) / size_of::<T>()
    }
}

pub struct SlabAllocator<T: SlabInit + 'static> {
    slab_frame_count: usize,
    slabs: RwLock<LinkedList<&'static mut Slab<T>>>,
}

impl<T: SlabInit> SlabAllocator<T> {
    pub const fn new(slab_frame_count: usize) -> Self {
        Self {
            slab_frame_count,
            slabs: RwLock::new(LinkedList::new()),
        }
    }

    pub fn alloc(&self) -> Option<SlabBox<T>> {
        for slab in self.slabs.read().iter() {
            if let Some(mut handle) = slab.alloc() {
                *handle = SlabInit::init()?;
                return Some(handle);
            }
        }

        let slab = Slab::new(self.slab_frame_count)?;
        let handle = slab.alloc();

        // Slabs has to stay valid for the rest of the kernel's life
        self.slabs.write().push_front(Box::leak(Box::new(slab)));
        let mut handle = handle?;
        *handle = SlabInit::init()?;
        Some(handle)
    }
}
