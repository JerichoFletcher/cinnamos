use crate::arch::context::Context;
use crate::println;

#[derive(Debug)]
pub enum TrapCause {
    Unknown,
    IllegalInstruction(usize),
    LoadMisaligned(usize),
    LoadFault(usize),
    StoreMisaligned(usize),
    StoreFault(usize),
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
            panic!("Illegal instruction (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, inst);
        }
        TrapCause::LoadMisaligned(addr) => {
            panic!("Load address misaligned (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, addr);
        }
        TrapCause::LoadFault(addr) => {
            panic!("Load access fault (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, addr);
        }
        TrapCause::StoreMisaligned(addr) => {
            panic!("Store address misaligned (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, addr);
        }
        TrapCause::StoreFault(addr) => {
            panic!("Store access fault (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, addr);
        }
        TrapCause::SupervisorEnvCall(fid) => {
            println!("S-mode env-call (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, fid);
            frame.ctx.set_pc(frame.ctx.pc() + size_of::<usize>());
        }
        TrapCause::UserEnvCall(fid) => {
            println!("U-mode env-call (at {:p}: 0x{:08x})", frame.ctx.pc() as *const u8, fid);
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
            panic!("Unexpected trap cause (pc={:p})", frame.ctx.pc() as *const u8);
        },
    }
}
