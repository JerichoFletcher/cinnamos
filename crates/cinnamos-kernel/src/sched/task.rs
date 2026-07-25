use spin::Mutex;

use crate::{
    arch::{self, VAddr},
    mem::{self, PhysFrameAlloc, alloc::slab::SlabAllocator, physalloc::Alloc},
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

    stack_alloc: Alloc,
}

impl Task {
    fn init(ptr: *mut Self) -> Result<(), ()> {
        // Safety: ptr is a valid pointer
        let val = unsafe { ptr.as_mut_unchecked() };
        val.state = TaskState::Ready;
        val.stack_alloc = mem::physalloc::alloc(2).ok_or(())?;
        val.kernel_stack_ptr = mem::vms::phys_to_virt(val.stack_alloc.start_addr());
        val.time_quantum = 0;
        Ok(())
    }

    fn deinit(ptr: *mut Self) {
        // Safety: ptr is a valid pointer
        let val = unsafe { ptr.read() };
        drop(val);
    }
}

struct SendAllocator(SlabAllocator<Task>);

unsafe impl Send for SendAllocator {}

static TASK_ALLOC: Mutex<SendAllocator> = Mutex::new(SendAllocator(SlabAllocator::new(
    1,
    Some(&Task::init),
    Some(&Task::deinit),
)));

pub fn new_kernel_task(entry: *const ()) -> *mut Task {
    let mut g = TASK_ALLOC.lock();
    let task = (*g).0.alloc();

    if !task.is_null() {
        // Safety: task is a valid pointer
        let task = unsafe { task.as_mut_unchecked() };

        let task_sp = mem::vms::phys_to_virt(task.stack_alloc.end_addr());
        task.kernel_stack_ptr = arch::create_task_init_stack(task.kernel_stack_ptr, entry, task_sp);
    }
    task
}
