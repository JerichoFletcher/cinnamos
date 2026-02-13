use core::sync::atomic::{AtomicBool, Ordering};
use crate::arch::time::has_timer;
use crate::cpu::local::{get_local, get_local_mut, CpuLocal};
use crate::println;
use crate::sched::task::{Task, TaskState};

pub const MAX_TASKS: usize = 32;

#[allow(dead_code)]
static mut TASKS: [Option<Task>; MAX_TASKS] = [const { None }; MAX_TASKS];
static DO_PREEMPT: AtomicBool = AtomicBool::new(false);

pub fn init() {
    let do_preempt = has_timer();
    DO_PREEMPT.store(do_preempt, Ordering::Relaxed);

    println!("Scheduler Max Tasks   : {}", MAX_TASKS);
    println!("Scheduler Preemption  : {}", do_preempt);
}

#[inline(always)]
pub fn should_preempt() -> bool {
    DO_PREEMPT.load(Ordering::Relaxed)
}

pub fn tick() {
    if should_preempt() {
        schedule()
    }
}

pub fn schedule() {
    let loc = get_local();
    let curr_task = loc.current_task();

    let loc = get_local_mut();
    let rq = loc.run_queue();

    if let Some(task) = curr_task {
        if let TaskState::Running = task.state {
            task.state = TaskState::Ready;
            rq.push(task);
        }
    }

    if rq.count() == 0 {
        panic!("No runnable tasks");
    }

    unsafe {
        match rq.pop().as_mut() {
            Some(task) => {
                task.state = TaskState::Running;
                loc.set_current_task(task);
            },
            None => {
                panic!("Run queue pops null task");
            }
        }
    }
}
