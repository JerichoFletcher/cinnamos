use crate::arch::context::Context;
use crate::println;

#[derive(Debug)]
pub enum TrapCause {
    Unknown,
    IllegalInstruction(usize),
    SupervisorEnvCall(usize),
    UserEnvCall(usize),
    TimerInterrupt,
    ExternalInterrupt,
    SoftwareInterrupt,
}

pub struct TrapFrame<'a, C : Context> {
    pub ctx: &'a mut C,
    pub cause: TrapCause,
}

pub fn ktrap_handle<C: Context>(frame: TrapFrame<C>) {
    match frame.cause {
        TrapCause::IllegalInstruction(inst) => {
            panic!("Illegal instruction 0x{:08x}: 0x{:08x}", frame.ctx.pc(), inst);
        }
        TrapCause::SupervisorEnvCall(fid) => {
            println!("Env-call (S): 0x{:08x}: 0x{:08x}", frame.ctx.pc(), fid);
            frame.ctx.set_pc(frame.ctx.pc() + size_of::<usize>());
        }
        TrapCause::UserEnvCall(fid) => {
            println!("Env-call (U): 0x{:08x}: 0x{:08x}", frame.ctx.pc(), fid);
            frame.ctx.set_pc(frame.ctx.pc() + size_of::<usize>());
        }
        TrapCause::TimerInterrupt => {
            println!("Timer interrupt");
        }
        TrapCause::ExternalInterrupt => {
            println!("External interrupt");
        }
        TrapCause::SoftwareInterrupt => {
            println!("Software interrupt");
        }
        _ => {
            panic!("Unexpected trap cause");
        },
    }
}
