use core::mem::offset_of;

use crate::{
    arch::{Task, VAddr}, mem::alloc::slab::SlabBox,
};

const _: () = debug_assert!(offset_of!(HartLocal, scratch) == 8);
const _: () = debug_assert!(offset_of!(HartLocal, curr_task_ptr) == 16);
const _: () = debug_assert!(offset_of!(HartLocal, trap_stack_top) == 24);

#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    /// Equals to the ID of the current hart.
    hid: usize,
    /// Scratch memory used for temporaries.
    scratch: usize,
    /// Equals to the [Task] pointed by `curr_task`, or null if it is [None].
    curr_task_ptr: *mut Task,
    /// Equals to the address of the trap stack top for the current hart.
    trap_stack_top: VAddr,
    /// The current task being executed by this hart.
    curr_task: Option<SlabBox<Task>>,
}

impl HartLocal {
    #[inline]
    pub fn new(hid: usize, tsp: VAddr) -> Self {
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

    #[inline]
    pub const fn curr_task(&mut self) -> Option<&mut SlabBox<Task>> {
        self.curr_task.as_mut()
    }

    #[inline]
    pub fn take_curr_task(&mut self) -> Option<SlabBox<Task>> {
        self.curr_task_ptr = core::ptr::null_mut();
        self.curr_task.take()
    }

    #[inline]
    pub fn set_curr_task(&mut self, task: SlabBox<Task>) {
        self.curr_task_ptr = task.as_ptr();
        self.curr_task = Some(task);
    }
}

#[inline]
pub fn load_hart_local(hloc: *const HartLocal) {
    unsafe {
        core::arch::asm!(
            "mv tp, {0}",
            in(reg) hloc as usize,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
pub fn hart_local() -> *mut HartLocal {
    let ptr: *mut HartLocal;
    unsafe {
        core::arch::asm!(
            "mv {0}, tp",
            out(reg) ptr,
            options(nomem, nostack, preserves_flags)
        );
        &mut *ptr
    }
}
