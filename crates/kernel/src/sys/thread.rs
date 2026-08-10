use cinnamos_abi::{SyscallError, proc::ThreadId};

use crate::sched;

pub fn thread_create(_entry: *const ()) -> Result<ThreadId, SyscallError> {
    todo!()
}

/// Yields execution of the current thread back to the operating system.
///
/// Upon reentrance, the thread will continue execution where it originally
/// calls this function.
pub fn thread_yield() -> Result<(), SyscallError> {
    sched::schedule();
    Ok(())
}

pub fn thread_exit(_exit_code: usize) -> ! {
    todo!()
}
