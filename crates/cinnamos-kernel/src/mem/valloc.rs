use crate::{arch::VAddr, *};

pub struct VirtualRegion {
    start: VAddr,
    end: VAddr,
    free_list: *mut [u8],
}

pub struct VirtualAllocator {

}
