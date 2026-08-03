#![no_std]

use cinnamos_abi_macros::SyscallTable;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

#[expect(unused)]
mod abi;

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
    #[returns(usize)]
    ProcessCreate = 1,

    #[args(exit_code: usize)]
    #[returns(!)]
    ProcessExit = 7,

    // THREAD
    #[args(entry: *const ())]
    #[returns(usize)]
    ThreadCreate = 8,

    ThreadYield = 9,

    #[args(exit_code: usize)]
    #[returns(!)]
    ThreadExit = 15,
}
