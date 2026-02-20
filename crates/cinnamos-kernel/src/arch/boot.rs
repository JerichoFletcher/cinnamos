#[inline]
#[unsafe(link_section = ".text.boot")]
pub fn init_pt(dtb_ptr: *const u8) -> usize {
    crate::arch::aimpl::boot::init_pt(dtb_ptr)
}
