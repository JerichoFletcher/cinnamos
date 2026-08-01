use spin::Once;

use crate::{
    arch::{VAddr, VMALLOC_MAP_BASE, VMALLOC_MAP_END},
    mem::virt::{
        VirtAllocator,
        buddy::{BuddyPageAlloc, BuddyVirtAllocator},
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
    get_vmalloc().alloc(page_count)
}
