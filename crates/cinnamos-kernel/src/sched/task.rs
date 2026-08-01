use crate::{
    arch::{self, VAddr},
    mem::{
        self, PhysFrameAlloc,
        alloc::slab::{SlabAllocator, SlabBox, SlabInit},
        physalloc::FrameAlloc,
    },
};

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
    pub id: usize,
    pub state: TaskState,
    pub kernel_stack_ptr: VAddr,
    pub time_quantum: usize,

    kernel_stack_alloc: FrameAlloc,
    task_stack_alloc: FrameAlloc,
}

impl SlabInit for Task {
    fn init() -> Option<Self> {
        let kernel_stack_alloc = mem::physalloc::alloc(2)?;
        let task_stack_alloc = mem::physalloc::alloc(16)?;

        let val = Self {
            id: 0,
            state: TaskState::Ready,
            kernel_stack_ptr: mem::vms::phys_to_virt(kernel_stack_alloc.start_addr()),
            time_quantum: 0,
            kernel_stack_alloc,
            task_stack_alloc,
        };
        Some(val)
    }
}

struct SendAllocator(SlabAllocator<1, Task>);

unsafe impl Sync for SendAllocator {}

static TASK_ALLOC: SendAllocator = SendAllocator(SlabAllocator::new());

pub fn new_kernel_task(entry: *const ()) -> Option<SlabBox<Task>> {
    let mut task = TASK_ALLOC.0.alloc()?;

    let task_sp = mem::vms::phys_to_virt(task.task_stack_alloc.end_addr());
    task.kernel_stack_ptr = arch::create_task_init_stack(task.kernel_stack_ptr, entry, task_sp);

    Some(task)
}
