use core::{arch::global_asm, mem::offset_of};

use crate::{hloc::HartLocal, sched::task::Task};

const _: () = debug_assert!(offset_of!(HartLocal, scratch) == 8);
const _: () = debug_assert!(offset_of!(HartLocal, task) == 16);
const _: () = debug_assert!(offset_of!(Task, kernel_stack_ptr) == 16);

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("trap.s"));
