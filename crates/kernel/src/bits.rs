pub const fn align_next(val: usize, order: usize) -> usize {
    let x: usize = (1 << order) - 1;
    (val + x) & !x
}
