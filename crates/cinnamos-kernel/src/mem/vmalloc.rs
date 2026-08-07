use spin::Once;

use crate::{
    arch::{VAddr, VMALLOC_MAP_BASE, VMALLOC_MAP_END},
    mem::{
        PAGE_SIZE,
        virt::{
            VirtAlloc, VirtAllocator,
            buddy::{BuddyPageAlloc, BuddyVirtAllocator},
        },
    },
};

static VMALLOC: Once<BuddyVirtAllocator> = Once::new();

/// A virtual page allocation.
pub type PageAlloc = BuddyPageAlloc;

fn get_vmalloc<'a>() -> &'a BuddyVirtAllocator {
    VMALLOC.call_once(|| {
        BuddyVirtAllocator::new(VAddr::new(VMALLOC_MAP_BASE), VAddr::new(VMALLOC_MAP_END))
    })
}

/// Allocate a number of virtual pages.
/// If allocation fails, this function returns [`None`].
pub fn alloc(page_count: usize) -> Option<PageAlloc> {
    let a = get_vmalloc().alloc(page_count);
    match &a {
        Some(a) => log::trace!(
            "allocate vpage size={} base={:#016x}",
            page_count * PAGE_SIZE,
            &a.start_addr()
        ),
        None => log::trace!("allocate vpage size={} failed", page_count * PAGE_SIZE),
    }
    a
}

/// Allocate a number of virtual pages, with a number of guard pages at the base.
///
/// Guard pages are included in the total size of the reserved virtual region, but do not count towards
/// the allocation region itself. This is useful for ensuring that an allocation has unmapped pages below
/// its region, which is useful for creating a stack, for example.
pub fn alloc_guarded(page_count: usize, guard_page_count: usize) -> Option<PageAlloc> {
    let a = get_vmalloc().alloc_guarded(page_count, guard_page_count);
    match &a {
        Some(a) => log::trace!(
            "allocate vpage size={} guard={} base={:#016x}",
            page_count * PAGE_SIZE,
            guard_page_count * PAGE_SIZE,
            &a.start_addr()
        ),
        None => log::trace!(
            "allocate vpage size={} guard={} failed",
            page_count * PAGE_SIZE,
            guard_page_count * PAGE_SIZE,
        ),
    }
    a
}
