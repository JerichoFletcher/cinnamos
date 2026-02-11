use crate::arch::context::Context;
use crate::cpu::idle;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TrapCause {
    Error,
    EnvCall,
    TimerInterrupt,
    ExternalInterrupt,
    SoftwareInterrupt,
}

pub fn ktrap_handle<C: Context>(_ctx: &mut C, cause: TrapCause) {
    match cause {
        TrapCause::Error => idle(),
        _ => {},
    }
}
