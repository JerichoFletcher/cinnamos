use crate::arch::{TrapFrame, VAddr};

pub enum TaskState {
    Ready,
    Running,
    Stopped,
}

#[repr(C)]
pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub kernel_stack_ptr: VAddr,
    pub frame: TrapFrame,
}
