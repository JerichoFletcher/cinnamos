#[inline(always)]
pub const fn u32_reverse_bytes(val: u32) -> u32 {
    ((val & 0xff00_0000) >> 24) |
    ((val & 0x00ff_0000) >> 16) |
    ((val & 0x0000_ff00) >> 8) |
    ((val & 0x0000_00ff))
}

#[inline(always)]
pub fn align_next_u64(val: u64, order: u8) -> u64 {
    let x = (1 << order) - 1;
    (val + x) & !x
}

#[inline(always)]
pub fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
