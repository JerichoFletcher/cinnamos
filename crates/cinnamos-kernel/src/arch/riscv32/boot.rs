#[inline]
#[unsafe(link_section = ".text.boot")]
pub fn init_pt(dtb_ptr: *const u8) -> usize {
    crate::arch::riscv32::vms::init_boot_pt(dtb_ptr)
}
