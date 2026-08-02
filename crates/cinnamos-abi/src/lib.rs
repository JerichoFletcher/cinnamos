#![no_std]

use macros::SyscallTable;
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

#[expect(unused)]
mod abi;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum SyscallError {
    // TODO: Define error codes
    #[num_enum(default)]
    UnknownError = 0xffff,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, SyscallTable)]
pub enum Syscall {
    // ProcessCreate = 1,
    #[args(exit_code: usize)]
    #[returns(!)]
    ProcessExit = 7,

    #[args(entry: *const ())]
    #[returns(usize)]
    ThreadCreate = 8,

    ThreadYield = 9,

    #[args(exit_code: usize)]
    #[returns(!)]
    ThreadExit = 15,
}
