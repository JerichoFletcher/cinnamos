use cinnamos_abi::proc::ThreadId;

use crate::{
    arch::{Context, PTEFlags, Task},
    mem::{
        self,
        alloc::slab::{SlabAllocator, SlabBox},
        physalloc::FrameAlloc,
        virt::VirtAlloc,
        vmalloc::PageAlloc,
    },
};

pub mod proc;

/// The state of a task.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// The task is ready to be run.
    Ready,
    /// The task is currently running.
    Running,
    /// The task has been terminated and can be safely reaped.
    Stopped,
}

/// A container for states that affect how a task is executed.
#[expect(unused)]
#[derive(Debug)]
pub struct TaskControlBlock {
    /// The ID of this task.
    pub id: ThreadId,
    /// The current state of this task.
    pub state: TaskState,
    /// The remaining time budget of this task.
    pub time_quantum: usize,

    kernel_stack_phys: FrameAlloc,
    task_stack_phys: FrameAlloc,

    kernel_stack_virt: PageAlloc,
    task_stack_virt: PageAlloc,
}

impl Task {
    /// Creates a new kernel task.
    fn new_kernel() -> Option<Self> {
        let kernel_stack_phys = mem::physalloc::alloc(31)?;
        let task_stack_phys = mem::physalloc::alloc(3)?;

        let kernel_stack_virt = mem::vmalloc::alloc_guarded(31, 1)?;
        let task_stack_virt = mem::vmalloc::alloc_guarded(3, 1)?;
        mem::vms::map(&kernel_stack_virt, &kernel_stack_phys, PTEFlags::GRW).ok()?;
        mem::vms::map(&task_stack_virt, &task_stack_phys, PTEFlags::GRW).ok()?;

        let csp = task_stack_virt.end_addr();
        let ksp = kernel_stack_virt.end_addr();
        let tcb = TaskControlBlock {
            id: 0.into(),
            state: TaskState::Ready,
            time_quantum: 0,
            kernel_stack_phys,
            task_stack_phys,
            kernel_stack_virt,
            task_stack_virt,
        };
        // Safety:
        Some(unsafe { Task::new(csp, ksp, tcb) })
    }
}

static TASK_ALLOC: SlabAllocator<4, Task> = SlabAllocator::new();

/// Creates a new kernel task with the given entry point.
///
/// The returned [Task] already has an initialized call stack in its kernel context
/// and can be safely [scheduled](crate::sched::schedule) directly.
pub fn new_kernel_task(entry: fn() -> !) -> Option<SlabBox<Task>> {
    let mut task = TASK_ALLOC.alloc(Task::new_kernel()?)?;
    // let task_sp = task.tcb().task_stack_virt.end_addr();

    // Safety: The allocated kernel stack fits the fabricated stack
    unsafe {
        task.build_stack()
            // Safety: entry points to executable code
            .push(Context::kernel_task_enter(entry));
    }
    Some(task)
}
