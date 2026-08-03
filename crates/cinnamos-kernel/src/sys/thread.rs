use cinnamos_abi::SyscallError;

use crate::{arch::IrqState, sched};

pub fn thread_create(_entry: *const ()) -> Result<usize, SyscallError> {
    todo!()
}

pub fn thread_yield() -> Result<(), SyscallError> {
    let irq = IrqState::disable_save();
    sched::schedule();
    irq.restore();
    Ok(())
}

pub fn thread_exit(_exit_code: usize) -> ! {
    todo!()
}
