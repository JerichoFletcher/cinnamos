#![no_std]

use cinnamos_abi_macros::SyscallTable;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

mod macros;
pub mod proc;

#[expect(unused)]
mod abi;
#[expect(unused)]
use crate::proc::*;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum SyscallError {
    UnknownSyscall = 0x7fff,

    #[num_enum(default)]
    UnknownError = 0xffff,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, SyscallTable)]
#[err(SyscallError::UnknownSyscall)]
pub enum Syscall {
    // PROCESS
    #[returns(ProcessId)]
    ProcessCreate = 1,

    #[args(exit_code: usize)]
    #[returns(!)]
    ProcessExit = 7,

    // THREAD
    #[args(entry: *const ())]
    #[returns(ThreadId)]
    ThreadCreate = 8,

    ThreadYield = 9,

    #[args(exit_code: usize)]
    #[returns(!)]
    ThreadExit = 15,
}
