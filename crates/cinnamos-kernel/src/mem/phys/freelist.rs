use core::{
    fmt::Debug,
    ptr::{self, NonNull},
};

use bitflags::bitflags;

use crate::{arch::PAddr, mem::PAGE_SIZE};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct FreeListEntry: u8 {
        const USED = 0b0000_0001;
        const RESERVED = 0b1000_0000;
    }
}

impl FreeListEntry {
    pub fn available(&self) -> bool {
        !self.intersects(Self::USED | Self::RESERVED)
    }
}

pub struct FreeListFrameAlloc {
    addr: PAddr,
    frame_count: usize,
}

impl super::FrameAlloc for FreeListFrameAlloc {
    fn base_addr(&self) -> PAddr {
        self.addr
    }
}

#[repr(C)]
pub struct FreeListAllocator {
    base_addr: PAddr,
    used: usize,
    free: usize,
    list: [FreeListEntry],
}

impl FreeListAllocator {
    /// # Safety
    /// - `start` and `end` must point to a valid, contiguous memory region.
    /// - `at` must be a page-aligned address that points to writable memory.
    pub unsafe fn create(at: PAddr, start: PAddr, end: PAddr) -> NonNull<Self> {
        let start_addr = (start.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end_addr = (end.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let size = (end_addr - start_addr) / PAGE_SIZE;

        let alloc: *mut FreeListAllocator = ptr::from_raw_parts_mut(at.addr() as *mut (), size);
        assert!(!alloc.is_null());
        unsafe {
            (&raw mut (*alloc).base_addr).write(PAddr::new(start_addr));
            (&raw mut (*alloc).used).write(0);
            (&raw mut (*alloc).free).write(size);

            for i in 0..size {
                (&raw mut (*alloc).list[i]).write(FreeListEntry::empty());
            }

            NonNull::new_unchecked(alloc)
        }
    }

    fn end_addr(&self) -> PAddr {
        self.base_addr + self.list.len() * PAGE_SIZE
    }
}

impl super::PhysFrameAllocator<FreeListFrameAlloc> for FreeListAllocator {
    fn size(start: PAddr, end: PAddr) -> usize {
        let start_addr = (start.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end_addr = (end.addr() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let size = (end_addr - start_addr) / PAGE_SIZE;

        size_of::<PAddr>() + size_of::<usize>() * 2 + size_of::<FreeListEntry>() * size
    }

    fn alloc(&mut self, size_bytes: usize) -> Option<FreeListFrameAlloc> {
        let frame_count = (size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        if frame_count == 0 || frame_count > self.free {
            return None;
        }

        let mut i: usize = 0;
        while i < self.list.len() {
            if self.list[i].available() {
                let mut usable = true;
                let mut j: usize = 1;
                while j < frame_count {
                    if !self.list[i + j].available() {
                        usable = false;
                        break;
                    } else {
                        j += 1;
                    }
                }

                if usable {
                    for k in i..(i + frame_count) {
                        self.list[k].insert(FreeListEntry::USED);
                    }
                    self.used += frame_count;
                    self.free -= frame_count;

                    return Some(FreeListFrameAlloc {
                        addr: self.base_addr + i * PAGE_SIZE,
                        frame_count,
                    });
                } else {
                    i += j;
                }
            } else {
                i += 1;
            }
        }

        None
    }

    fn dealloc(&mut self, handle: &mut FreeListFrameAlloc) {
        if self.base_addr <= handle.addr && handle.addr < self.end_addr() {
            let i = (handle.addr - self.base_addr) / PAGE_SIZE;
            for j in 0..handle.frame_count {
                if !self.list[i + j].contains(FreeListEntry::USED) {
                    panic!(
                        "Attempted to deallocate unused frame i={} (0x{:016x})",
                        i + j,
                        (handle.addr + (i + j) * PAGE_SIZE)
                    );
                }
            }

            for j in 0..handle.frame_count {
                self.list[i + j].remove(FreeListEntry::USED);
            }
            self.used -= handle.frame_count;
            self.free += handle.frame_count;
        }
    }

    fn reserve(&mut self, start: PAddr, end: PAddr) {
        if start < self.end_addr() && end > self.base_addr && start < end {
            let start = if start < self.base_addr {
                self.base_addr
            } else {
                start
            };
            let end = if end > self.end_addr() {
                self.end_addr()
            } else {
                end
            };

            let i_start = (((start + PAGE_SIZE - 1).addr() & !(PAGE_SIZE - 1))
                - self.base_addr.addr())
                / PAGE_SIZE;
            let i_endx = (((end + PAGE_SIZE - 1).addr() & !(PAGE_SIZE - 1))
                - self.base_addr.addr())
                / PAGE_SIZE;

            for i in i_start..i_endx {
                self.list[i].set(FreeListEntry::RESERVED, true);
            }
            self.free -= i_endx - i_start;
        }
    }
}
