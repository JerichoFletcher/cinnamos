use core::mem::MaybeUninit;

use crate::{
    arch::{self, VAddr},
    mem::alloc::slab::SlabBox,
    task::Task,
};

#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    pub(crate) hid: usize,
    pub(crate) scratch: usize,
    pub(crate) curr_task_ptr: *mut Task,
    pub(crate) trap_stack_top: VAddr,

    curr_task: Option<SlabBox<Task>>,
}

impl HartLocal {
    fn new(hid: usize, tsp: VAddr) -> Self {
        Self {
            hid,
            scratch: 0,
            curr_task_ptr: core::ptr::null_mut(),
            trap_stack_top: tsp,
            curr_task: None,
        }
    }

    #[inline]
    pub const fn hid(&self) -> usize {
        self.hid
    }

    pub const fn curr_task(&mut self) -> Option<&mut SlabBox<Task>> {
        self.curr_task.as_mut()
    }

    pub fn take_curr_task(&mut self) -> Option<SlabBox<Task>> {
        self.curr_task_ptr = core::ptr::null_mut();
        self.curr_task.take()
    }

    pub fn set_curr_task(&mut self, task: SlabBox<Task>) {
        self.curr_task_ptr = task.as_ptr();
        self.curr_task = Some(task);
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
