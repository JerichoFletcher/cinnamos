use core::{mem::offset_of, sync::atomic::AtomicUsize};

use crate::{
    arch::{Task, VAddr}, mem::alloc::slab::SlabBox,
};

const _: () = debug_assert!(offset_of!(HartLocal, scratch) == 8);
const _: () = debug_assert!(offset_of!(HartLocal, curr_task_ptr) == 16);
const _: () = debug_assert!(offset_of!(HartLocal, trap_stack_top) == 24);

/// The storage object that is exclusive for each hart.
#[repr(C)]
#[derive(Debug)]
pub struct HartLocal {
    /// Equals to the ID of the current hart.
    hid: usize,
    /// Scratch memory used for temporaries.
    scratch: usize,
    /// Equals to the [Task] pointed by `curr_task`, or null if it is [`None`](None).
    curr_task_ptr: *mut Task,
    /// Equals to the address of the trap stack top for the current hart.
    trap_stack_top: VAddr,
    /// The current task being executed by this hart.
    curr_task: Option<SlabBox<Task>>,

    /// The nesting level of critical sections by this hart.
    critical_nesting: AtomicUsize,
}

impl HartLocal {
    /// Creates a new hart-local storage object. This does not load it into the hart state.
    #[inline]
    pub fn new(hid: usize, tsp: VAddr) -> Self {
        Self {
            hid,
            scratch: 0,
            curr_task_ptr: core::ptr::null_mut(),
            trap_stack_top: tsp,
            curr_task: None,
            critical_nesting: AtomicUsize::new(0),
        }
    }

    /// The ID of the current hart. Since hart ID is practically a constant, it is always safe to read.
    #[inline]
    pub const fn hid(&self) -> usize {
        self.hid
    }

    /// The current task being executed by the current hart, if any.
    #[inline]
    pub const fn curr_task(&mut self) -> Option<&mut SlabBox<Task>> {
        self.curr_task.as_mut()
    }

    /// Takes the current task out of the storage.
    #[inline]
    pub fn take_curr_task(&mut self) -> Option<SlabBox<Task>> {
        self.curr_task_ptr = core::ptr::null_mut();
        self.curr_task.take()
    }

    /// Saves a task into the storage, returning the previous one if it exists.
    #[inline]
    pub fn set_curr_task(&mut self, mut task: SlabBox<Task>) -> Option<SlabBox<Task>> {
        self.curr_task_ptr = task.as_mut() as *mut _;
        self.curr_task.replace(task)
    }

    /// The nesting level of critical sections by this hart.
    #[inline]
    pub const fn critical_nesting(&self) -> &AtomicUsize {
        &self.critical_nesting
    }
}

/// Installs a pointer to a hart-local storage into the hart state.
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

/// Gets the pointer to the hart-local storage associated with this hart.
#[inline]
pub fn hart_local() -> *mut HartLocal {
    let ptr: *mut HartLocal;
    unsafe {
        core::arch::asm!(
            "mv {0}, tp",
            out(reg) ptr,
            options(nomem, nostack, preserves_flags)
        );
        ptr
    }
}
