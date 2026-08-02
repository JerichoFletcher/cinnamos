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

pub type PageAlloc = BuddyPageAlloc;

fn get_vmalloc<'a>() -> &'a BuddyVirtAllocator {
    VMALLOC.call_once(|| {
        BuddyVirtAllocator::new(VAddr::new(VMALLOC_MAP_BASE), VAddr::new(VMALLOC_MAP_END))
    })
}

pub fn alloc(page_count: usize) -> Option<BuddyPageAlloc> {
    let a = get_vmalloc().alloc(page_count);
    match &a {
        Some(a) => log::trace!(
            "allocate vpage size={} base=0x{:016x}",
            page_count * PAGE_SIZE,
            &a.start_addr()
        ),
        None => log::trace!("allocate vpage size={} failed", page_count * PAGE_SIZE),
    }
    a
}
