use cinnamos_abi::{Syscall, SyscallError};
use cinnamos_kernel_macros::gen_syscall_dispatch;

pub mod proc;
pub mod thread;

use proc::*;
use thread::*;

cinnamos_abi::__syscall_meta!(gen_syscall_dispatch);
