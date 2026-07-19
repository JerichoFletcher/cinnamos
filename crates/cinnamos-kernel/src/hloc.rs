use core::mem::MaybeUninit;

use crate::{
    arch::{self, VAddr},
    *,
};

#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    pub hid: usize,
    pub kernel_stack_ptr: VAddr,
    pub scratch: usize,
}

impl HartLocal {
    fn new(hid: usize, kernel_stack_ptr: VAddr) -> Self {
        Self {
            kernel_stack_ptr,
            hid,
            scratch: 0,
        }
    }
}

static mut BOOT_HLOC: MaybeUninit<HartLocal> = MaybeUninit::zeroed();

/// Should only be called by the boot hart
#[inline]
pub fn load_boot_hart_local(hid: usize) {
    unsafe {
        let ptr = &raw mut (BOOT_HLOC) as *mut HartLocal;
        ptr.write(HartLocal::new(hid, trap_stack_end_v!()));
        arch::load_boot_hart_local(ptr);
    }
}

/// # Safety
/// This function can only be safely called after the [HartLocal](HartLocal) for the caller has been loaded.
#[inline]
pub unsafe fn hart_local() -> &'static mut HartLocal {
    unsafe { arch::hart_local() }
}
