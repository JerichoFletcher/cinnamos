use core::mem::MaybeUninit;

use crate::{arch::{self, VAddr}, sched::task::Task};

#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    pub hid: usize,
    pub scratch: usize,
    pub curr_task: *mut Task,
    pub trap_stack_top: VAddr,
}

impl HartLocal {
    fn new(hid: usize, tsp: VAddr) -> Self {
        Self {
            hid,
            scratch: 0,
            curr_task: core::ptr::null_mut(),
            trap_stack_top: tsp,
        }
    }

    #[inline]
    pub const fn hid(&self) -> usize {
        self.hid
    }

    pub const fn curr_task(&mut self) -> Option<&mut Task> {
        // Safety: self is mutably borrowed
        unsafe { self.curr_task.as_mut() }
    }

    pub const fn set_curr_task(&mut self, task: *mut Task) {
        self.curr_task = task;
    }
}

static mut BOOT_HLOC: MaybeUninit<HartLocal> = MaybeUninit::zeroed();

/// Should only be called by the boot hart
#[inline]
pub fn load_boot_hart_local(hid: usize, tsp: VAddr) {
    let ptr = &raw mut (BOOT_HLOC) as *mut HartLocal;
    unsafe {
        ptr.write(HartLocal::new(hid, tsp));
    }
    arch::load_boot_hart_local(ptr);
}

/// Should only be called after the [HartLocal](HartLocal) for the caller has been loaded.
#[inline]
pub fn hart_local() -> &'static mut HartLocal {
    unsafe { arch::hart_local() }
}
