use crate::sched::MAX_TASKS;
use crate::sched::task::Task;

pub struct RunQueue {
    tasks: [Option<*mut Task>; MAX_TASKS],
    count: usize,
    head: usize,
    tail: usize,
}

impl RunQueue {
    pub const fn new() -> Self {
        Self {
            tasks: [None; MAX_TASKS],
            count: 0,
            head: 0,
            tail: 0,
        }
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn push(&mut self, task: *mut Task) {
        assert!(self.count < MAX_TASKS);

        self.tasks[self.tail] = Some(task);
        self.count += 1;
        self.tail = (self.tail + 1) % MAX_TASKS;
    }

    pub fn pop(&mut self) -> *mut Task {
        assert!(self.count > 0);
        
        let task = self.tasks[self.head].expect("Invalid run queue state; null task");
        self.tasks[self.head] = None;
        self.count -= 1;
        self.head = (self.head + 1) % MAX_TASKS;
        task
    }
}
