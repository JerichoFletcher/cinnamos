use riscv::interrupt::{Trap, Interrupt, Exception};
use riscv::register::*;
use crate::arch::riscv::context::{RiscvContext, RiscvRawContext};
use crate::cpu::interrupt::InterruptMask;
use crate::kernel::trap::{ktrap_handle, TrapCause};

#[repr(C)]
struct RiscvTrapFrame {
    pub ctx: RiscvRawContext,
}

fn setup_trap_vector() {
    unsafe {
        unsafe extern "C" {
            fn _trap_vector();
        }
        let tvec = stvec::Stvec::new(
            _trap_vector as *const () as usize,
            stvec::TrapMode::Direct
        );
        stvec::write(tvec);
    }
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut RiscvTrapFrame) {
    let raw_trap = scause::read().cause();
    let trap: Trap<Interrupt, Exception> = raw_trap.try_into().unwrap();

    let mut kctx = RiscvContext::new(&mut frame.ctx);
    let kcause = match trap {
        Trap::Exception(Exception::UserEnvCall) => TrapCause::EnvCall,
        Trap::Interrupt(Interrupt::SupervisorTimer) => TrapCause::TimerInterrupt,
        Trap::Interrupt(Interrupt::SupervisorExternal) => TrapCause::ExternalInterrupt,
        Trap::Interrupt(Interrupt::SupervisorSoft) => TrapCause::SoftwareInterrupt,
        _ => TrapCause::Error,
    };
    ktrap_handle(&mut kctx, kcause);
}

pub fn init() {
    setup_trap_vector();
}

pub fn set_interrupt_mask(mask: &InterruptMask) {
    let mut ie = sie::read();
    ie.set_stimer(mask.timer);
    ie.set_sext(mask.external);
    ie.set_ssoft(mask.software);
    unsafe {
        sie::write(ie);
    }
}
