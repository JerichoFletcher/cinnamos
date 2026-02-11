#[cfg(target_arch = "riscv32")]
use crate::arch::riscv::console::RiscvSbiConsole as ConsoleImpl;

pub trait Console {
    fn putchar(c: u8) -> Result<(), ()>;
    fn getchar() -> Result<u8, ()>;
    fn putstr(s: &str) -> Result<(), ()>;
}

#[inline(always)]
pub fn putchar(c: u8) -> Result<(), ()> {
    ConsoleImpl::putchar(c)
}

#[inline(always)]
pub fn getchar() -> Result<u8, ()> {
    ConsoleImpl::getchar()
}

#[inline(always)]
pub fn putstr(s: &str) -> Result<(), ()> {
    ConsoleImpl::putstr(s)
}
