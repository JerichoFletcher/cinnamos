use core::{arch::global_asm, mem::offset_of};

use crate::{
    arch::{SWITCH_FRAME_SIZE, TrapFrame},
    hloc::HartLocal,
    sched::task::Task,
};

const _: () = debug_assert!(size_of::<TrapFrame>() == (36 * size_of::<usize>()));
const _: () = debug_assert!(offset_of!(HartLocal, scratch) == 8);
const _: () = debug_assert!(offset_of!(HartLocal, curr_task) == 16);
const _: () = debug_assert!(offset_of!(Task, kernel_stack_ptr) == 16);
const _: () = debug_assert!(SWITCH_FRAME_SIZE == 13 * 8);

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("trap.s"));
global_asm!(include_str!("sched.s"));
