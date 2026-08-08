use cinnamos_abi::{SyscallError, proc::ProcessId};

pub fn process_create() -> Result<ProcessId, SyscallError> {
    todo!()
}

pub fn process_exit(_exit_code: usize) -> ! {
    todo!()
}
