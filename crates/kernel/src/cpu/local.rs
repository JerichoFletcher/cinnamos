use crate::arch::cpu::CpuLocalImpl;
use crate::sched::queue::RunQueue;
use crate::sched::task::Task;

pub static MAX_CPU: usize = 1;

pub trait CpuLocal {
    fn id(&self) -> usize;

    fn current_task(&self) -> Option<&mut Task>;
    fn set_current_task(&mut self, task: &mut Task);

    fn kernel_sp(&self) -> usize;
    fn set_kernel_sp(&mut self, sp: usize);

    fn next_deadline(&self) -> u64;
    fn set_next_deadline(&mut self, next_deadline: u64);

    fn run_queue(&mut self) -> &mut RunQueue;
}

#[inline(always)]
pub fn get_local() -> &'static CpuLocalImpl {
    crate::arch::cpu::local()
}

#[inline(always)]
pub fn get_local_mut() -> &'static mut CpuLocalImpl {
    crate::arch::cpu::local_mut()
}
