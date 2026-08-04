use core::{arch::global_asm, mem::offset_of};

use crate::{
    arch::{Context, TrapFrame},
    hloc::HartLocal,
    task::Task,
};

const _: () = debug_assert!(size_of::<TrapFrame>() == (36 * size_of::<usize>()));
const _: () = debug_assert!(size_of::<Context>() == 13 * size_of::<usize>());
const _: () = debug_assert!(offset_of!(HartLocal, scratch) == 8);
const _: () = debug_assert!(offset_of!(HartLocal, curr_task_ptr) == 16);
const _: () = debug_assert!(offset_of!(HartLocal, trap_stack_top) == 24);
const _: () = debug_assert!(offset_of!(Task, context_sp) == 16);
const _: () = debug_assert!(offset_of!(Task, kernel_stack_ptr) == 24);

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("trap.s"));
global_asm!(include_str!("sched.s"));
