use riscv::asm::wfi;

#[inline]
pub fn wait_for_interrupt() {
    wfi();
}
