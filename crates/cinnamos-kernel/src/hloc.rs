use core::mem::MaybeUninit;

use crate::{arch, sched::task::Task};

#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    pub hid: usize,
    pub scratch: usize,
    pub task: *mut Task,
}

impl HartLocal {
    fn new(hid: usize) -> Self {
        Self {
            hid,
            scratch: 0,
            task: core::ptr::null_mut(),
        }
    }
}

static mut BOOT_HLOC: MaybeUninit<HartLocal> = MaybeUninit::zeroed();

/// Should only be called by the boot hart
#[inline]
pub fn load_boot_hart_local(hid: usize) {
    unsafe {
        let ptr = &raw mut (BOOT_HLOC) as *mut HartLocal;
        ptr.write(HartLocal::new(hid));
        arch::load_boot_hart_local(ptr);
    }
}

/// Should only be called after the [HartLocal](HartLocal) for the caller has been loaded.
#[inline]
pub fn hart_local() -> &'static mut HartLocal {
    unsafe { arch::hart_local() }
}
