use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::vec_deque::VecDeque;

use crate::{
    arch::{self, Task},
    hloc,
    mem::alloc::slab::SlabBox,
    sync::mutex_irq::MutexIrq,
    task::TaskState,
};

// improper_ctypes is fine since only the fields with specified layouts are accessed from assembly code.
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

/// A task scheduler responsible for managing which and when tasks should be executed.
pub struct Scheduler {
    next_free_id: AtomicUsize,
    run_queue: MutexIrq<VecDeque<SlabBox<Task>>>,
}

impl Scheduler {
    /// Adds a task to the scheduler.
    ///
    /// The scheduler will assign a [`ThreadId`](cinnamos_abi::ThreadId) and set the task's status to
    /// [`Ready`](TaskState::Ready). The task will be available to be scheduled for eventual execution.
    ///
    /// # Safety
    /// `task` must be in an enterable state, i.e. it has an initialized call stack loaded into its context.
    pub unsafe fn enqueue(&self, mut task: SlabBox<Task>) {
        let next_free_id = self.next_free_id.fetch_add(1, Ordering::Relaxed);
        log::trace!("enqueueing task id={}", next_free_id);
        let tcb = task.tcb_mut();
        tcb.id = next_free_id.into();
        tcb.state = TaskState::Ready;
        tcb.time_quantum = 0;

        arch::interrupt_free(|ms| {
            self.run_queue.lock(ms).push_back(task);
        });
    }

    /// Yields execution of the current task and switches to the next one.
    ///
    /// Will disable interrupts for the hart until the function returns.
    pub fn schedule(&self) {
        arch::interrupt_free(|ms| {
            let mut hloc = hloc::borrow(ms);
            let mut rq = self.run_queue.lock(ms);
            if rq.is_empty() {
                return;
            }

            let mut curr = hloc
                .take_curr_task()
                .expect("Scheduler::schedule() expects a current task");
            let tcb = curr.tcb_mut();
            if tcb.state == TaskState::Running {
                tcb.state = TaskState::Ready;
            }

            let curr_ptr = curr.as_mut() as *mut _;
            let curr_id = curr.tcb().id;
            rq.push_back(curr);
            let mut next = rq.pop_front().unwrap();

            let tcb = next.tcb_mut();
            tcb.state = TaskState::Running;
            if tcb.time_quantum == 0 {
                // TODO: Priority-based quantum budget assignment
                tcb.time_quantum = 5;
            }
            let next_ptr = next.as_ref() as *const _;
            log::trace!(
                "scheduling next task curr={} next={}",
                curr_id,
                next.tcb().id
            );
            drop(rq);

            hloc.set_curr_task(next);
            // Safety: Both pointers are derived from allocated slab boxes, and
            // the next task has a call stack in its context
            unsafe { __switch(next_ptr, curr_ptr) };
        });
    }

    /// Starts execution of the scheduler.
    ///
    /// # Safety
    /// Since this function may enable interrupts for idle harts:
    /// - Do not call this function inside a critical section.
    /// - If this function is called from an ISR, the interrupt flag must be disabled.
    pub unsafe fn start(&self) -> ! {
        loop {
            arch::interrupt_free(|ms| {
                let mut hloc = hloc::borrow(ms);
                let next = self.run_queue.lock(ms).pop_front();
                if let Some(mut next) = next {
                    let tcb = next.tcb_mut();
                    tcb.state = TaskState::Running;
                    if tcb.time_quantum == 0 {
                        // TODO: Priority-based quantum budget assignment
                        tcb.time_quantum = 5;
                    }
                    let next_ptr = next.as_ref() as *const _;
                    log::trace!("scheduling first task next={}", next.tcb().id);

                    hloc.set_curr_task(next);
                    // Safety: The pointer is derived from an allocated slab box, and
                    // the next task has a call stack in its context
                    unsafe { __switch_noprev(next_ptr) };
                    unreachable!("__switch_noprev should never return to Scheduler::start()");
                }
            });
            // Safety: Outside of critical section
            unsafe { self.wait() };
        }
    }

    /// Safety: Must not be called within a critical section or ISR.
    unsafe fn wait(&self) {
        unsafe {
            arch::interrupt_nested(|| {
                arch::wait_for_interrupt();
            })
        }
    }
}

// Safety: Queue mutations are synchronized with critical sections and mutexes
unsafe impl Sync for Scheduler {}

static GLOBAL_SCHEDULER: Scheduler = Scheduler {
    next_free_id: AtomicUsize::new(0),
    run_queue: MutexIrq::new(VecDeque::new()),
};

/// Starts execution of the kernel global scheduler.
///
/// # Safety
/// Since this function may enable interrupts for idle harts:
/// - Do not call this function inside a critical section.
/// - If this function is called from an ISR, the interrupt flag must be disabled.
pub unsafe fn start() -> ! {
    unsafe { GLOBAL_SCHEDULER.start() }
}

/// Adds a task to the kernel global scheduler.
///
/// The scheduler will assign a [`ThreadId`](cinnamos_abi::ThreadId) and set the task's status to
/// [`Ready`](TaskState::Ready). The task will be available to be scheduled for eventual execution.
///
/// # Safety
/// `task` must be in an enterable state, i.e. it has an initialized call stack loaded into its context.
pub unsafe fn enqueue(task: SlabBox<Task>) {
    // Safety: task has an initialized call stack
    unsafe { GLOBAL_SCHEDULER.enqueue(task) };
}

/// Yields execution of the current task and switches to the next one.
///
/// Because this function accesses the hart-local storage, it is executed in a critical section.
pub fn schedule() {
    GLOBAL_SCHEDULER.schedule();
}
