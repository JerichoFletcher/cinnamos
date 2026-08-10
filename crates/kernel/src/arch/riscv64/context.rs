use riscv::register::sstatus;

use crate::arch::{addr::VAddr, riscv64::task::kernel_task_enter};

const _: () = debug_assert!(size_of::<Context>() == 21 * size_of::<usize>());

/// Represents a context within a call stack.
#[repr(C)]
#[derive(Debug)]
pub struct Context {
    /// The return address from the current context.
    pub ra: VAddr,
    /// The contents of callee-saved registers.
    pub saved: [usize; 20],
}

impl Context {
    pub const REG_S0: usize = 0;
    pub const REG_S1: usize = 1;
    pub const REG_S2: usize = 2;
    pub const REG_S3: usize = 3;
    pub const REG_S4: usize = 4;
    pub const REG_S5: usize = 5;
    pub const REG_S6: usize = 6;
    pub const REG_S7: usize = 7;
    pub const REG_S8: usize = 8;
    pub const REG_S9: usize = 9;
    pub const REG_S10: usize = 10;
    pub const REG_S11: usize = 11;
    pub const REG_A0: usize = 12;
    pub const REG_A1: usize = 13;
    pub const REG_A2: usize = 14;
    pub const REG_A3: usize = 15;
    pub const REG_A4: usize = 16;
    pub const REG_A5: usize = 17;
    pub const REG_A6: usize = 18;
    pub const REG_A7: usize = 19;

    /// Creates an empty context with the given return address.
    #[inline]
    pub const fn new(ret: VAddr) -> Self {
        Self {
            ra: ret,
            saved: [0; 20],
        }
    }

    /// Creates a context that returns to [`task::kernel_task_enter`](crate::arch::task::kernel_task_enter).
    #[inline]
    pub fn kernel_task_enter(entry: fn() -> !) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_sie(true);

        let mut saved = [0; 20];
        saved[Self::REG_A0] = entry as _;
        saved[Self::REG_A1] = sstatus.bits();
        Self {
            ra: VAddr::from_ptr(kernel_task_enter as *const ()),
            saved,
        }
    }
}
