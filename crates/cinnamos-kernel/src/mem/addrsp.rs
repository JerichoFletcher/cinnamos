use alloc::vec::Vec;

use crate::{
    arch::{PAddr, PageTable},
    mem::{PhysFrameAlloc, palloc::Alloc},
};

#[derive(Debug)]
pub struct AddressSpace {
    root_ptr: *mut PageTable,
    root: Alloc,
    tables: Vec<Alloc>,
}

impl AddressSpace {
    pub const fn take(root_ptr: *mut PageTable, root: Alloc, tables: Vec<Alloc>) -> Self {
        Self {
            root_ptr,
            root,
            tables,
        }
    }

    pub const fn root_ptr(&self) -> *mut PageTable {
        self.root_ptr
    }

    pub fn root_pa(&self) -> PAddr {
        self.root.start_addr()
    }
}
