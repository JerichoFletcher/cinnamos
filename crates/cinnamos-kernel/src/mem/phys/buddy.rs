use core::{alloc::Layout, ptr::NonNull};

use structs::buddy::BuddyTreeAllocator;

use crate::arch::PAddr;

pub struct BuddyFrameAllocator {
    base_addr: PAddr,
    tree: NonNull<BuddyTreeAllocator>,
}

impl BuddyFrameAllocator {
    
}
