use core::ptr::null_mut;
use crate::arch::cpu::Cpu;
use crate::cpu::local::{CpuLocal, MAX_CPU};
use crate::sched::queue::RunQueue;
use crate::sched::task::Task;

static mut CPUS: [RiscvCpuLocal; MAX_CPU] = [RiscvCpuLocal::new(); MAX_CPU];

#[repr(C)]
pub struct RiscvCpuLocal {
    hid: usize,
    kernel_sp: usize,
    scratch: usize,
    current_task: *mut Task,
    next_deadline: u64,
    run_queue: RunQueue,
}

impl CpuLocal for RiscvCpuLocal {
    fn id(&self) -> usize {
        self.hid
    }

    fn current_task(&self) -> Option<&mut Task> {
        unsafe { self.current_task.as_mut() }
    }

    fn set_current_task(&mut self, task: &mut Task) {
        self.current_task = task as *mut Task
    }

    fn kernel_sp(&self) -> usize {
        self.kernel_sp
    }

    fn set_kernel_sp(&mut self, sp: usize) {
        self.kernel_sp = sp;
    }

    fn next_deadline(&self) -> u64 {
        self.next_deadline
    }

    fn set_next_deadline(&mut self, next_deadline: u64) {
        self.next_deadline = next_deadline;
    }

    fn run_queue(&mut self) -> &mut RunQueue {
        &mut self.run_queue
    }
}

impl RiscvCpuLocal {
    pub const fn new() -> Self {
        Self {
            hid: 0,
            kernel_sp: 0,
            scratch: 0,
            current_task: null_mut(),
            run_queue: RunQueue::new(),
            next_deadline: 0,
        }
    }
}

pub struct RiscvCpu;

impl Cpu<RiscvCpuLocal> for RiscvCpu {
    fn init(id: usize) {
        let cpu = unsafe {
            &mut CPUS[id]
        };
        cpu.hid = id;
        unsafe {
            core::arch::asm!(
                "mv tp, {cpu}",
                "csrw sscratch, {cpu}",
                cpu = in(reg) cpu as *mut _ as usize
            );
        }
    }

    #[inline(always)]
    fn local() -> &'static RiscvCpuLocal {
        let ptr: *const RiscvCpuLocal;
        unsafe {
            core::arch::asm!("mv {}, tp", out(reg) ptr);
            &*ptr
        }
    }

    #[inline(always)]
    fn local_mut() -> &'static mut RiscvCpuLocal {
        let ptr: *mut RiscvCpuLocal;
        unsafe {
            core::arch::asm!("mv {}, tp", out(reg) ptr);
            &mut *ptr
        }
    }

    #[inline(always)]
    fn idle() -> ! {
        loop {
            riscv::asm::wfi();
        }
    }
}
