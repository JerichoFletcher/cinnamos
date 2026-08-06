use cinnamos_abi::{Syscall, SyscallError};
use riscv::{
    interrupt::{Exception, Interrupt, Trap}, register::{
        scause::Scause, sscratch, sstatus::{self, Sstatus}, stvec::{self, Stvec, TrapMode},
    },
};

use crate::{
    arch::{self, Context, VAddr, interrupt}, hloc::HartLocalGuard, *,
};

const _: () = debug_assert!(size_of::<TrapFrame>() == (36 * size_of::<usize>()));

unsafe extern "C" {
    fn __trap_entry() -> !;
    fn __trap_exit() -> !;
}

#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    regs: [usize; 32],
    sstatus: Sstatus,
    sepc: VAddr,
    scause: Scause,
    stval: usize,
}

impl TrapFrame {
    pub const REG_RA: usize = 1;
    pub const REG_SP: usize = 2;
    pub const REG_GP: usize = 3;
    pub const REG_TP: usize = 4;
    pub const REG_T0: usize = 5;
    pub const REG_T1: usize = 6;
    pub const REG_T2: usize = 7;
    pub const REG_S0: usize = 8;
    pub const REG_S1: usize = 9;
    pub const REG_A0: usize = 10;
    pub const REG_A1: usize = 11;
    pub const REG_A2: usize = 12;
    pub const REG_A3: usize = 13;
    pub const REG_A4: usize = 14;
    pub const REG_A5: usize = 15;
    pub const REG_A6: usize = 16;
    pub const REG_A7: usize = 17;
    pub const REG_S2: usize = 18;
    pub const REG_S3: usize = 19;
    pub const REG_S4: usize = 20;
    pub const REG_S5: usize = 21;
    pub const REG_S6: usize = 22;
    pub const REG_S7: usize = 23;
    pub const REG_S8: usize = 24;
    pub const REG_S9: usize = 25;
    pub const REG_S10: usize = 26;
    pub const REG_S11: usize = 27;
    pub const REG_T3: usize = 28;
    pub const REG_T4: usize = 29;
    pub const REG_T5: usize = 30;
    pub const REG_T6: usize = 31;

    fn create_kernel_frame(entry: *const (), stack_ptr: VAddr) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spie(true);
        sstatus.set_spp(sstatus::SPP::Supervisor);

        let mut regs = [0; 32];
        regs[Self::REG_SP] = stack_ptr.addr();
        regs[Self::REG_TP] = hloc::get().as_ptr() as usize;
        Self {
            regs,
            sstatus,
            sepc: VAddr::from_ptr(entry),
            scause: Scause::from_bits(0),
            stval: 0,
        }
    }
}

pub fn create_init_trap_frame(entry: *const (), task_sp: VAddr) -> TrapFrame {
    TrapFrame::create_kernel_frame(entry, task_sp)
}

pub fn create_init_context() -> Context {
    Context::new(VAddr::from_ptr(__trap_exit as *const ()))
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let mut hloc = hloc::get();
    let tcause = frame
        .scause
        .cause()
        .try_into::<Interrupt, Exception>()
        .expect("Invalid trap cause");

    match tcause {
        Trap::Exception(Exception::InstructionMisaligned) => panic!(
            "[at {:#016x}] Instruction misaligned {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::InstructionFault) => panic!(
            "[at {:#016x}] Instruction fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::IllegalInstruction) => panic!(
            "[at {:#016x}] Illegal instruction {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadMisaligned) => panic!(
            "[at {:#016x}] Load misaligned {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadFault) => panic!(
            "[at {:#016x}] Load fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::StoreMisaligned) => panic!(
            "[at {:#016x}] Store misaligned {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::StoreFault) => panic!(
            "[at {:#016x}] Store fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::InstructionPageFault) => panic!(
            "[at {:#016x}] Instruction page fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadPageFault) => panic!(
            "[at {:#016x}] Load page fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::StorePageFault) => panic!(
            "[at {:#016x}] Store page fault {:#016x}",
            frame.sepc, frame.stval
        ),
        Trap::Exception(Exception::UserEnvCall) => {
            process_syscall(frame);
            frame.sepc = frame.sepc + 4;
        }
        Trap::Exception(Exception::Breakpoint) => {
            frame.sepc = frame.sepc + 4;
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            arch::timer::schedule_timer();
            if let Some(curr) = hloc.curr_task() {
                let t = curr.tcb().time_quantum;
                if t == 0 {
                    sched::schedule();
                } else {
                    curr.tcb_mut().time_quantum -= 1;
                }
            }
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            handle_external_interrupt(&mut hloc);
        }
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            log::trace!(
                "[at {:#016x}] Software interrupt {:#016x}",
                frame.sepc, frame.stval
            );
        }
        _ => panic!(
            "[at {:#016x}] Unhandled trap {:?} {:#016x}",
            frame.sepc, tcause, frame.stval
        ),
    }
}

fn process_syscall(frame: &mut TrapFrame) {
    match Syscall::try_from(frame.regs[TrapFrame::REG_A7]) {
        Ok(sys) => {
            if let Some(args) = frame.regs[TrapFrame::REG_A0..TrapFrame::REG_A5].first_chunk::<6>() {
                // Safety: args passed from the context are generated from their original types
                match unsafe { sys::dispatch_syscall(sys, args) } {
                    Ok(ret) => {
                        log::trace!("syscall OK sys={:?} ret={}", sys, ret);
                        frame.regs[TrapFrame::REG_A0] = 0;
                        frame.regs[TrapFrame::REG_A1] = ret;
                    }
                    Err(e) => {
                        log::trace!("syscall ERR sys={:?} err={:?}", sys, e);
                        frame.regs[TrapFrame::REG_A0] = e.into();
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("syscall ERR sys={} unknown", e.number);
            frame.regs[TrapFrame::REG_A0] = SyscallError::UnknownSyscall.into();
        }
    }
}

fn handle_external_interrupt(hloc: &mut HartLocalGuard) {
    if let Some(claim) = arch::device::plic::claim_irq(hloc.hid()) {
        let irq = claim.irq_id();
        if let Err(e) = interrupt::dispatch_irq(irq) {
            log::warn!(
                "HID {} failed to handle claimed interrupt {}: {:?}",
                hloc.hid(),
                irq,
                e
            );
        }
    }
}

pub fn init() {
    let trap_entry_addr = __trap_entry as *const () as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::get().as_ptr() as usize);
    }
}

pub fn init_higher_half() {
    let trap_entry_addr = __trap_entry as *const () as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::get().as_ptr() as usize);
    }
}
