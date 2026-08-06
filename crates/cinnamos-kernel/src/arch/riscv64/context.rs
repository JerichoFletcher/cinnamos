use crate::arch::VAddr;

const _: () = debug_assert!(size_of::<Context>() == 13 * size_of::<usize>());

#[repr(C)]
#[derive(Debug)]
pub struct Context {
    pub ra: VAddr,
    pub saved: [usize; 12],
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

    pub const fn new(ret: VAddr) -> Self {
        Self { ra: ret, saved: [0; 12] }
    }
}
