use core::mem::offset_of;

use crate::{arch::VAddr, task::TaskControlBlock, util::mem::stack::StackBuilder};

const _: () = debug_assert!(offset_of!(Task, context_sp) == 0);
const _: () = debug_assert!(offset_of!(Task, kernel_sp) == 8);

/// A task struct representing a runnable thread.
#[repr(C)]
#[derive(Debug)]
pub struct Task {
    context_sp: VAddr,
    kernel_sp: VAddr,
    tcb: TaskControlBlock,
}

impl Task {
    /// Creates a new task.
    ///
    /// # Safety
    /// `context_sp` and `kernel_sp` must point to writable, mapped stack memory.
    #[inline]
    pub const unsafe fn new(context_sp: VAddr, kernel_sp: VAddr, tcb: TaskControlBlock) -> Self {
        Self {
            context_sp,
            kernel_sp,
            tcb,
        }
    }

    /// Creates a [`TaskStackBuilder`] on the task's working stack.
    #[inline]
    pub const fn build_stack<'a>(&'a mut self) -> TaskStackBuilder<'a> {
        let stack = StackBuilder::new(self.context_sp);
        TaskStackBuilder { task: self, stack }
    }

    /// Gets a reference to the [`TaskControlBlock`] for this task.
    #[inline]
    pub const fn tcb(&self) -> &TaskControlBlock {
        &self.tcb
    }

    /// Gets a mutable reference to the [`TaskControlBlock`] for this task.
    #[inline]
    pub const fn tcb_mut(&mut self) -> &mut TaskControlBlock {
        &mut self.tcb
    }
}

/// Provides methods to push data into the working stack of a [`Task`].
pub struct TaskStackBuilder<'a> {
    task: &'a mut Task,
    stack: StackBuilder,
}

impl TaskStackBuilder<'_> {
    /// Pushes a value into the stack, e.g. writes the value into memory and shifts
    /// down the stack pointer.
    ///
    /// # Safety
    /// There must be enough space within the task stack, below the current pointer,
    /// to fit an aligned instance of `T`.
    pub unsafe fn push<T>(&mut self, val: T) -> &mut Self {
        unsafe {
            self.stack.push(val);
        }
        self.task.context_sp = self.stack.get();
        self
    }
}
