use crate::{
    arch::{self, PTEFlags, VAddr}, mem::{
        self, PAGE_SIZE, PhysFrameAlloc, alloc::slab::{SlabAllocator, SlabBox, SlabInit}, physalloc::FrameAlloc, virt::VirtAlloc, vmalloc::PageAlloc,
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
    pub context_sp: VAddr,
    pub kernel_stack_ptr: VAddr,
    pub time_quantum: usize,

    kernel_stack_phys: FrameAlloc,
    task_stack_phys: FrameAlloc,

    kernel_stack_virt: PageAlloc,
    task_stack_virt: PageAlloc,
}

impl SlabInit for Task {
    fn init() -> Option<Self> {
        let kernel_stack_phys = mem::physalloc::alloc(4)?;
        let task_stack_phys = mem::physalloc::alloc(32)?;

        let kernel_stack_virt = mem::vmalloc::alloc(kernel_stack_phys.frame_count() - 2)?;
        let task_stack_virt = mem::vmalloc::alloc(task_stack_phys.frame_count() - 2)?;
        mem::vms::map_raw(
            kernel_stack_virt.start_addr() + PAGE_SIZE * 2,
            kernel_stack_phys.start_addr(),
            kernel_stack_virt.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        ).ok()?;
        mem::vms::map_raw(
            task_stack_virt.start_addr() + PAGE_SIZE * 2,
            task_stack_phys.start_addr(),
            task_stack_virt.size(),
            PTEFlags::GLOBAL | PTEFlags::RW,
        ).ok()?;

        let val = Self {
            id: 0,
            state: TaskState::Ready,
            kernel_stack_ptr: kernel_stack_virt.end_addr() + PAGE_SIZE * 2,
            context_sp: kernel_stack_virt.end_addr() + PAGE_SIZE * 2,
            time_quantum: 0,
            kernel_stack_phys,
            task_stack_phys,
            kernel_stack_virt,
            task_stack_virt,
        };
        Some(val)
    }
}

struct SendAllocator(SlabAllocator<1, Task>);

unsafe impl Sync for SendAllocator {}

static TASK_ALLOC: SendAllocator = SendAllocator(SlabAllocator::new());

pub fn new_kernel_task(entry: *const ()) -> Option<SlabBox<Task>> {
    let mut task = TASK_ALLOC.0.alloc()?;

    let task_sp = task.task_stack_virt.end_addr() + PAGE_SIZE * 2;
    task.context_sp = arch::create_task_init_stack(task.kernel_stack_ptr, entry, task_sp);

    Some(task)
}
