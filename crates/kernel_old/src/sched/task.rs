use crate::arch::context::{Context, ContextImpl};
use crate::cpu::PrivMode;

#[derive(Debug)]
pub enum IrqType {
    External,
}

#[derive(Debug)]
pub enum TaskState {
    Ready,
    Running,
    BlockedIrq(IrqType),
}

pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub ctx: ContextImpl,
    pub priority: usize,
    pub quantum: usize,
}

impl Task {
    pub fn new(id: usize, privilege: PrivMode, priority: usize) -> Self {
        Self {
            id,
            state: TaskState::Ready,
            ctx: ContextImpl::new(privilege),
            priority,
            quantum: 0,
        }
    }
}
