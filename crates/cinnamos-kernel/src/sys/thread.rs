use cinnamos_abi::{SyscallError, proc::ThreadId};

use crate::{arch::IrqState, sched};

pub fn thread_create(_entry: *const ()) -> Result<ThreadId, SyscallError> {
    todo!()
}

pub fn thread_yield() -> Result<(), SyscallError> {
    let irq = IrqState::save_disable();
    sched::schedule();
    irq.restore();
    Ok(())
}

pub fn thread_exit(_exit_code: usize) -> ! {
    todo!()
}
