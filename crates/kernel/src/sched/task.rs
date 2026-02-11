use crate::arch::context::Context;

#[derive(Debug)]
pub enum IrqType {
    External,
}

#[derive(Debug)]
pub enum TaskState {
    NotCreated,
    Ready,
    Running,
    BlockedIrq(IrqType),
    Terminated,
}

pub struct Task<C : Context> {
    pub id: usize,
    pub state: TaskState,
    pub ctx: C,
    pub priority: usize,
}
