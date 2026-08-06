use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::vec_deque::VecDeque;
use spin::Mutex;

use crate::{
    hloc,
    mem::alloc::slab::SlabBox,
    task::{Task, TaskState},
};

#[expect(improper_ctypes)]
unsafe extern "C" {
    fn __switch(next_task: *const Task, curr_task: *mut Task);
    fn __switch_noprev(next_task: *const Task);
}

pub struct Scheduler {
    next_free_id: AtomicUsize,
    run_queue: Mutex<VecDeque<SlabBox<Task>>>,
}

impl Scheduler {
    pub fn enqueue(&self, mut task: SlabBox<Task>) {
        let next_free_id = self.next_free_id.fetch_add(1, Ordering::Relaxed);
        log::trace!(
            "enqueueing task ptr={:p} id={}",
            task.as_ptr(),
            next_free_id
        );
        task.id = next_free_id.into();
        task.state = TaskState::Ready;
        task.time_quantum = 0;
        self.run_queue.lock().push_back(task);
    }

    pub fn schedule(&self) {
        let mut hloc = hloc::hart_local();
        let mut curr = hloc
            .take_curr_task()
            .expect("Scheduler::schedule() expects a current task");
        if curr.state == TaskState::Running {
            curr.state = TaskState::Ready;
        }
        let curr_ptr = curr.as_ptr();
        let curr_id = curr.id;

        let next = {
            let mut rq = self.run_queue.lock();
            rq.push_back(curr);
            rq.pop_front()
        };
        match next {
            Some(mut next) => {
                next.state = TaskState::Running;
                if next.time_quantum == 0 {
                    // TODO: Priority-based quantum budget assignment
                    next.time_quantum = 5;
                }

                let next_ptr = next.as_ptr();
                log::trace!("scheduling next task curr={} next={}", curr_id, next.id);

                hloc.set_curr_task(next);
                unsafe {
                    __switch(next_ptr, curr_ptr);
                }
            }
            None => {
                panic!("schedule run queue is empty")
            }
        }
    }

    pub fn start(&self) -> ! {
        let mut hloc = hloc::hart_local();
        let next = self.run_queue.lock().pop_front();
        match next {
            Some(mut next) => {
                next.state = TaskState::Running;
                if next.time_quantum == 0 {
                    // TODO: Priority-based quantum budget assignment
                    next.time_quantum = 5;
                }

                let next_ptr = next.as_ptr();
                log::trace!("scheduling first task next={}", next.id);

                hloc.set_curr_task(next);
                unsafe {
                    __switch_noprev(next_ptr);
                }
                unreachable!("__switch_noprev should never return to Scheduler::start()");
            }
            None => panic!("schedule run queue is empty"),
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
