use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::vec_deque::VecDeque;
use spin::Mutex;

use crate::{
    hloc,
    mem::alloc::slab::SlabBox,
    task::{Task, TaskState},
};

unsafe extern "C" {
    fn __switch(next_task: *const (), curr_task: *mut ());
    fn __switch_noprev(next_task: *const ());
}

pub struct Scheduler {
    next_free_id: AtomicUsize,
    run_queue: Mutex<VecDeque<SlabBox<Task>>>,
}

impl Scheduler {
    pub fn enqueue(&self, mut task: SlabBox<Task>) {
        log::trace!("enqueueing task ptr={:p}", task.as_ptr());
        let next_free_id = self.next_free_id.fetch_add(1, Ordering::Relaxed);
        task.id = next_free_id;
        task.state = TaskState::Ready;
        task.time_quantum = 128;

        let mut rq = self.run_queue.lock();
        rq.push_back(task);
    }

    pub fn schedule(&self) {
        let hloc = hloc::hart_local();

        let mut rq = self.run_queue.lock();
        let curr = hloc
            .curr_task()
            .expect("Scheduler::schedule() expects a current task");
        if curr.state == TaskState::Running {
            curr.state = TaskState::Ready;
        }

        match rq.pop_front() {
            Some(mut next) => {
                next.state = TaskState::Running;
                let next_ptr = next.as_ptr();
                log::trace!(
                    "scheduling next task curr={:p} next={:p}",
                    curr,
                    next.as_ptr()
                );

                rq.push_back(next);
                drop(rq);

                let curr_ptr = curr as *mut Task;
                hloc.set_curr_task(next_ptr);
                unsafe {
                    __switch(next_ptr as _, curr_ptr as _);
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

        match rq.pop_front() {
            Some(mut next) => {
                let next_ptr = next.as_ptr();
                next.state = TaskState::Running;
                log::trace!("scheduling first task next={:p}", next.as_ptr());

                rq.push_back(next);
                drop(rq);

                hloc.set_curr_task(next_ptr);
                unsafe {
                    __switch_noprev(next_ptr as _);
                }
                unreachable!("__switch_noprev should never return to Scheduler::start()");
            }
            None => {
                drop(rq);
                panic!("Schedule run queue is empty")
            }
        }
    }
}

unsafe impl Sync for Scheduler {}

static GLOBAL_SCHEDULER: Scheduler = Scheduler {
    next_free_id: AtomicUsize::new(0),
    run_queue: Mutex::new(VecDeque::new()),
};

pub fn start() -> ! {
    GLOBAL_SCHEDULER.start()
}

pub fn enqueue(task: SlabBox<Task>) {
    GLOBAL_SCHEDULER.enqueue(task);
}

pub fn schedule() {
    GLOBAL_SCHEDULER.schedule();
}
