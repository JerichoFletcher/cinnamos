#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::sbi::console_putchar as arch_putchar;
#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::sbi::console_getchar as arch_getchar;

pub fn putchar(c: u8) {
    arch_putchar(c);
}

pub fn getchar() -> u8 {
    arch_getchar()
}

pub fn putstr(s: &str) {
    for b in s.bytes() {
        arch_putchar(b);
    }
}

pub fn clear() {
    putstr("\x1b[2J\x1b[1;1H");
}
