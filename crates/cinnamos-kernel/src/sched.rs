use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::vec_deque::VecDeque;
use spin::Mutex;

use crate::{arch::Task, hloc, mem::alloc::slab::SlabBox, task::TaskState};

#[expect(improper_ctypes)]
unsafe extern "C" {
    /// Saves the current execution context to `curr_task` and loads a new one from `next_task`.
    ///
    /// # Safety
    /// - `curr_task` and `next_task` must point to valid [Task] objects.
    /// - `next_task` must have an initialized call stack.
    fn __switch(next_task: *const Task, curr_task: *mut Task);

    /// Immediately loads the execution context from `next_task`.
    ///
    /// # Safety
    /// `next_task` must point to a valid [Task] object with an initialized call stack.
    fn __switch_noprev(next_task: *const Task);
}

pub struct Scheduler {
    next_free_id: AtomicUsize,
    run_queue: Mutex<VecDeque<SlabBox<Task>>>,
}

impl Scheduler {
    /// # Safety
    /// `task` must have an initialized call stack loaded into its context.
    pub unsafe fn enqueue(&self, mut task: SlabBox<Task>) {
        let next_free_id = self.next_free_id.fetch_add(1, Ordering::Relaxed);
        log::trace!(
            "enqueueing task ptr={:p} id={}",
            task.as_ptr(),
            next_free_id
        );
        let tcb = task.tcb_mut();
        tcb.id = next_free_id.into();
        tcb.state = TaskState::Ready;
        tcb.time_quantum = 0;
        self.run_queue.lock().push_back(task);
    }

    pub fn schedule(&self) {
        let mut hloc = hloc::get();
        let mut curr = hloc
            .take_curr_task()
            .expect("Scheduler::schedule() expects a current task");
        let tcb = curr.tcb_mut();
        if tcb.state == TaskState::Running {
            tcb.state = TaskState::Ready;
        }

        let curr_ptr = curr.as_ptr();
        let curr_id = curr.tcb().id;

        let next = {
            let mut rq = self.run_queue.lock();
            rq.push_back(curr);
            rq.pop_front()
        };
        match next {
            Some(mut next) => {
                let tcb = next.tcb_mut();
                tcb.state = TaskState::Running;
                if tcb.time_quantum == 0 {
                    // TODO: Priority-based quantum budget assignment
                    tcb.time_quantum = 5;
                }
                let next_ptr = next.as_ptr();
                log::trace!(
                    "scheduling next task curr={} next={}",
                    curr_id,
                    next.tcb().id
                );

                hloc.set_curr_task(next);
                // Safety: Both pointers are derived from allocated slab boxes, and
                // the next task has a call stack in its context
                unsafe { __switch(next_ptr, curr_ptr) };
            }
            None => {
                panic!("schedule run queue is empty")
            }
        }
    }

    pub fn start(&self) -> ! {
        let mut hloc = hloc::get();
        let next = self.run_queue.lock().pop_front();
        match next {
            Some(mut next) => {
                let tcb = next.tcb_mut();
                tcb.state = TaskState::Running;
                if tcb.time_quantum == 0 {
                    // TODO: Priority-based quantum budget assignment
                    tcb.time_quantum = 5;
                }
                let next_ptr = next.as_ptr();
                log::trace!("scheduling first task next={}", next.tcb().id);

                hloc.set_curr_task(next);
                // Safety: The pointer is derived from an allocated slab box, and
                // the next task has a call stack in its context
                unsafe { __switch_noprev(next_ptr) };
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

/// # Safety
/// `task` must have an initialized call stack loaded into its context.
pub unsafe fn enqueue(task: SlabBox<Task>) {
    // Safety: task has an initialized call stack
    unsafe { GLOBAL_SCHEDULER.enqueue(task) };
}

pub fn schedule() {
    GLOBAL_SCHEDULER.schedule();
}
