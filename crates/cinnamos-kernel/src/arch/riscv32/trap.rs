use riscv::interrupt::{Exception, Interrupt, Trap};
use riscv::register::{scause, stvec};

use crate::arch::riscv32::context::RiscvContext;
use crate::trap::{TrapCause, TrapFrame, ktrap_handle};

#[repr(C)]
struct RiscvTrapFrame {
    pub ctx: RiscvContext,
    pub scause: usize,
    pub stval: usize,
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut RiscvTrapFrame) {
    let tval = frame.stval;
    let tcause = scause::Scause::from_bits(frame.scause)
        .cause().try_into::<Interrupt, Exception>().unwrap();
    let kcause = match tcause {
        Trap::Exception(Exception::IllegalInstruction) => TrapCause::IllegalInstruction(tval),
        Trap::Exception(Exception::LoadMisaligned) => TrapCause::LoadMisaligned(tval),
        Trap::Exception(Exception::LoadFault) => TrapCause::LoadFault(tval),
        Trap::Exception(Exception::StoreMisaligned) => TrapCause::StoreMisaligned(tval),
        Trap::Exception(Exception::StoreFault) => TrapCause::StoreFault(tval),
        Trap::Exception(Exception::SupervisorEnvCall) => TrapCause::SupervisorEnvCall(frame.ctx.regs[17]),
        Trap::Exception(Exception::UserEnvCall) => TrapCause::UserEnvCall(frame.ctx.regs[17]),
        Trap::Interrupt(Interrupt::SupervisorTimer) => TrapCause::TimerInterrupt,
        Trap::Interrupt(Interrupt::SupervisorExternal) => TrapCause::ExternalInterrupt,
        Trap::Interrupt(Interrupt::SupervisorSoft) => TrapCause::SoftwareInterrupt,
        _ => TrapCause::Unknown,
    };
    ktrap_handle(TrapFrame {
        ctx: &mut frame.ctx,
        cause: kcause,
    });
}

pub fn init() {
    // Install trap vector
    unsafe {
        unsafe extern "C" {
            fn _trap_vector();
        }
        let addr = _trap_vector as *const() as usize;
        let tvec = stvec::Stvec::new(addr, stvec::TrapMode::Direct);
        stvec::write(tvec);
    }
}
