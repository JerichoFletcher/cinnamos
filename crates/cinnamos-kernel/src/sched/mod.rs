use alloc::collections::vec_deque::VecDeque;
use spin::Mutex;

use crate::{
    hloc,
    sched::task::{Task, TaskState},
};

pub mod task;

unsafe extern "C" {
    fn __switch(next_ksp: *const (), curr_ksp: *mut ());
    fn __switch_noprev(next_ksp: *const ());
}

pub struct Scheduler {
    run_queue: Mutex<(usize, VecDeque<*mut Task>)>,
}

impl Scheduler {
    pub const EMPTY: Self = Self {
        run_queue: Mutex::new((0, VecDeque::new())),
    };

    pub fn enqueue(&self, task: *mut Task) {
        if !task.is_null() && task.is_aligned() {
            let mut rq = self.run_queue.lock();

            // Safety: task points to a valid task
            let curr = unsafe { task.as_mut_unchecked() };
            curr.id = (*rq).0;
            (*rq).0 += 1;

            curr.state = TaskState::Ready;
            curr.time_quantum = 128;
            (*rq).1.push_back(task);
        }
    }

    pub fn schedule(&self) {
        let hloc = hloc::hart_local();
        let mut rq = self.run_queue.lock();

        let curr = hloc.curr_task().expect("schedule() expects a current task");
        if curr.state == TaskState::Running {
            curr.state = TaskState::Ready;
        }
        (*rq).1.push_back(curr);

        match (*rq).1.pop_front() {
            Some(next) => {
                // Safety: The scheduler run queue always stores valid pointers to ready tasks
                unsafe {
                    (*next).state = TaskState::Running;
                }
                drop(rq);
                let curr_ptr = curr as *mut Task;
                hloc.set_curr_task(next);

                unsafe {
                    __switch(next as *const _, curr_ptr as *mut _);
                }
            }
            None => {
                drop(rq);
                panic!("Schedule run queue is empty")
            }
        }
    }

    pub fn start(&self) -> ! {
        let hloc = hloc::hart_local();
        let mut rq = self.run_queue.lock();

        match (*rq).1.pop_front() {
            Some(next) => {
                // Safety: The scheduler run queue always stores valid pointers to ready tasks
                unsafe {
                    (*next).state = TaskState::Running;
                }
                drop(rq);
                hloc.set_curr_task(next);

                unsafe {
                    __switch_noprev(next as *const _);
                }
                panic!("__switch_noprev returned to Scheduler::start");
            }
            None => {
                drop(rq);
                panic!("Schedule run queue is empty")
            }
        }
    }
}

unsafe impl Sync for Scheduler {}

static GLOBAL_SCHEDULER: Scheduler = Scheduler::EMPTY;

pub fn start() -> ! {
    GLOBAL_SCHEDULER.start()
}

pub fn enqueue(task: *mut Task) {
    GLOBAL_SCHEDULER.enqueue(task);
}

pub fn schedule() {
    GLOBAL_SCHEDULER.schedule();
}
