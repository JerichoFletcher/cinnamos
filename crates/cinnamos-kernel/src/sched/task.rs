use crate::arch::Context;

pub enum TaskState {
    Ready,
    Running,
    Stopped,
}

pub struct Task {
    id: usize,
    state: TaskState,
    ctx: Context,
}
