use riscv::interrupt::{Trap, Interrupt, Exception};
use riscv::register::{stvec, sscratch, sie, scause};
use crate::arch::trap::Trap as TrapIntf;
use crate::arch::riscv::context::RiscvContext;
use crate::cpu::interrupt::InterruptMask;
use crate::trap::{ktrap_handle, TrapCause, TrapFrame};

#[repr(C)]
struct RiscvTrapFrame {
    pub ctx: RiscvContext,
    pub scause: usize,
    pub stval: usize,
}

impl RiscvTrapFrame {
    pub fn trap_cause(&self) -> Trap<Interrupt, Exception> {
        let raw_trap = scause::Scause::from_bits(self.scause).cause();
        raw_trap.try_into().unwrap()
    }

    pub fn trap_value(&self) -> usize {
        self.stval
    }
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut RiscvTrapFrame) {
    let tval = frame.trap_value();
    let kcause = match frame.trap_cause() {
        Trap::Exception(Exception::IllegalInstruction) => TrapCause::IllegalInstruction(tval),
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

#[inline(always)]
fn install_trap_vector() {
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

#[inline(always)]
fn init_scratch() {
    unsafe {
        unsafe extern "C" {
            static _trap_stack_top: u8;
        }
        let tsp = &_trap_stack_top as *const u8;
        sscratch::write(tsp as usize);
    }
}

pub fn init() {
    install_trap_vector();
    init_scratch();
}

pub struct RiscvTrap;

impl TrapIntf for RiscvTrap {
    fn set_interrupt_mask(mask: &InterruptMask) {
        let mut ie = sie::read();
        ie.set_stimer(mask.timer);
        ie.set_sext(mask.external);
        ie.set_ssoft(mask.software);
        unsafe {
            sie::write(ie);
        }
    }
}
