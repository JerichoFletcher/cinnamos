use cinnamos_abi::{Syscall, SyscallError};
use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause::Scause,
        sscratch, sstatus,
        stvec::{self, Stvec, TrapMode},
    },
};

use crate::{
    arch::{self, SWITCH_FRAME_SIZE, VAddr, context::Context, interrupt},
    hloc::HartLocal,
    *,
};

unsafe extern "C" {
    fn __trap_entry() -> !;
    fn __trap_exit() -> !;
}

#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    ctx: Context,
    scause: Scause,
    stval: usize,
}

impl TrapFrame {
    fn create_kernel_frame(entry: *const (), stack_ptr: VAddr) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spie(true);
        sstatus.set_spp(sstatus::SPP::Supervisor);

        let mut regs = [0; 32];
        regs[Context::REG_SP] = stack_ptr.addr();
        regs[Context::REG_TP] = hloc::hart_local() as *mut _ as usize;
        Self {
            ctx: Context {
                regs,
                sstatus,
                sepc: VAddr::from_ptr(entry),
            },
            scause: Scause::from_bits(0),
            stval: 0,
        }
    }
}

pub fn create_task_init_stack(at: VAddr, entry: *const (), task_sp: VAddr) -> VAddr {
    let mut at = at - size_of::<TrapFrame>();
    unsafe {
        at.as_mut::<TrapFrame>()
            .write(TrapFrame::create_kernel_frame(entry, task_sp));
    }
    at = at - SWITCH_FRAME_SIZE;
    unsafe {
        at.as_mut::<*const ()>().write(__trap_exit as *const ());
    }
    at
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame, hloc: &mut HartLocal) {
    let tcause = frame
        .scause
        .cause()
        .try_into::<Interrupt, Exception>()
        .expect("Invalid trap cause");

    match tcause {
        Trap::Exception(Exception::InstructionMisaligned) => panic!(
            "[at 0x{:016x}] Instruction misaligned 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::InstructionFault) => panic!(
            "[at 0x{:016x}] Instruction fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::IllegalInstruction) => panic!(
            "[at 0x{:016x}] Illegal instruction 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadMisaligned) => panic!(
            "[at 0x{:016x}] Load misaligned 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadFault) => panic!(
            "[at 0x{:016x}] Load fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::StoreMisaligned) => panic!(
            "[at 0x{:016x}] Store misaligned 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::StoreFault) => panic!(
            "[at 0x{:016x}] Store fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::InstructionPageFault) => panic!(
            "[at 0x{:016x}] Instruction page fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::LoadPageFault) => panic!(
            "[at 0x{:016x}] Load page fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::StorePageFault) => panic!(
            "[at 0x{:016x}] Store page fault 0x{:016x}",
            frame.ctx.sepc, frame.stval
        ),
        Trap::Exception(Exception::UserEnvCall) => {
            process_syscall(frame);
            frame.ctx.sepc = frame.ctx.sepc + 4;
        }
        Trap::Exception(Exception::Breakpoint) => {
            frame.ctx.sepc = frame.ctx.sepc + 4;
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            arch::timer::schedule_timer();
            if let Some(curr) = hloc.curr_task() {
                if curr.time_quantum == 0 {
                    curr.time_quantum = 128;
                    sched::schedule();
                } else {
                    curr.time_quantum -= 1;
                }
            }
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            handle_external_interrupt(hloc);
        }
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            log::trace!(
                "[at 0x{:016x}] Software interrupt 0x{:016x}",
                frame.ctx.sepc, frame.stval
            );
        }
        _ => panic!(
            "[at 0x{:016x}] Unhandled trap {:?} 0x{:016x}",
            frame.ctx.sepc, tcause, frame.stval
        ),
    }
}

fn process_syscall(frame: &mut TrapFrame) {
    match Syscall::try_from(frame.ctx.regs[Context::REG_A7]) {
        Ok(sys) => {
            if let Some(args) = frame.ctx.regs[Context::REG_A0..Context::REG_A5].first_chunk::<6>() {
                // Safety: args passed from the context are generated from their original types
                match unsafe { sys::dispatch_syscall(sys, args) } {
                    Ok(ret) => {
                        log::trace!("syscall OK sys={:?} ret={}", sys, ret);
                        frame.ctx.regs[Context::REG_A0] = 0;
                        frame.ctx.regs[Context::REG_A1] = ret;
                    }
                    Err(e) => {
                        log::trace!("syscall ERR sys={:?} err={:?}", sys, e);
                        frame.ctx.regs[Context::REG_A0] = e.into();
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("syscall ERR sys={} unknown", e.number);
            frame.ctx.regs[Context::REG_A0] = SyscallError::UnknownSyscall.into();
        }
    }
}

fn handle_external_interrupt(hloc: &mut HartLocal) {
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
        sscratch::write(hloc::hart_local() as *const _ as usize);
    }
}

pub fn init_higher_half() {
    let trap_entry_addr = __trap_entry as *const () as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::hart_local() as *const _ as usize);
    }
}
