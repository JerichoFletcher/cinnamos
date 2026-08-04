use cinnamos_abi::proc::ThreadId;

use crate::{
    arch::{self, PTEFlags, VAddr},
    mem::{
        self,
        alloc::slab::{SlabAllocator, SlabBox},
        physalloc::FrameAlloc,
        virt::VirtAlloc,
        vmalloc::PageAlloc,
    },
    util::stack::StackBuilder,
};

pub mod proc;

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Stopped,
}

#[repr(C)]
#[derive(Debug)]
pub struct Task {
    pub(crate) id: ThreadId,
    pub(crate) state: TaskState,
    pub(crate) context_sp: VAddr,
    pub(crate) kernel_stack_ptr: VAddr,
    pub(crate) time_quantum: usize,

    kernel_stack_phys: FrameAlloc,
    task_stack_phys: FrameAlloc,

    kernel_stack_virt: PageAlloc,
    task_stack_virt: PageAlloc,
}

impl Task {
    fn new_kernel() -> Option<Self> {
        let kernel_stack_phys = mem::physalloc::alloc(3)?;
        let task_stack_phys = mem::physalloc::alloc(31)?;

        let kernel_stack_virt = mem::vmalloc::alloc_guarded(3, 1)?;
        let task_stack_virt = mem::vmalloc::alloc_guarded(31, 1)?;
        mem::vms::map(
            &kernel_stack_virt,
            &kernel_stack_phys,
            PTEFlags::GLOBAL | PTEFlags::RW,
        )
        .ok()?;
        mem::vms::map(
            &task_stack_virt,
            &task_stack_phys,
            PTEFlags::GLOBAL | PTEFlags::RW,
        )
        .ok()?;

        let val = Self {
            id: 0.into(),
            state: TaskState::Ready,
            kernel_stack_ptr: kernel_stack_virt.end_addr(),
            context_sp: kernel_stack_virt.end_addr(),
            time_quantum: 0,
            kernel_stack_phys,
            task_stack_phys,
            kernel_stack_virt,
            task_stack_virt,
        };
        Some(val)
    }
}

static TASK_ALLOC: SlabAllocator<4, Task> = SlabAllocator::new();

pub fn new_kernel_task(entry: *const ()) -> Option<SlabBox<Task>> {
    let mut task = TASK_ALLOC.alloc(Task::new_kernel()?)?;

    let task_sp = task.task_stack_virt.end_addr();
    // Safety: The allocated kernel stack fits the fabricated stack
    task.context_sp = unsafe {
        StackBuilder::new(task.kernel_stack_ptr)
            .push(arch::create_init_trap_frame(entry, task_sp))
            .push(arch::create_init_context())
            .get()
    };
    Some(task)
}
