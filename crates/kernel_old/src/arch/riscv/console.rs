use sbi_rt::Physical;
use crate::arch::console::Console;
use crate::arch::riscv::sbi::SBI_CAPS;

pub struct RiscvSbiConsole;

impl Console for RiscvSbiConsole {
    fn putchar(c: u8) -> Result<(), ()> {
        if SBI_CAPS.has_debug_console() {
            match sbi_rt::console_write_byte(c).into_result() {
                Ok(_) => Ok(()),
                Err(_) => Err(())
            }
        } else {
            #[allow(deprecated)]
            match sbi_rt::legacy::console_putchar(c as usize) {
                0 => Ok(()),
                _ => Err(())
            }
        }
    }

    fn getchar() -> Result<u8, ()> {
        if SBI_CAPS.has_debug_console() {
            let mut tmp: [u8; 1] = [0];
            let slice = Physical::new(tmp.len(), tmp.as_mut_ptr() as usize, 0);
            match sbi_rt::console_read(slice).into_result() {
                Ok(_) => Ok(tmp[0]),
                Err(_) => Err(())
            }
        } else {
            #[allow(deprecated)]
            match sbi_rt::legacy::console_getchar() {
                usize::MAX => Err(()),
                val => Ok(val as u8),
            }
        }
    }

    fn putstr(s: &str) -> Result<(), ()> {
        if SBI_CAPS.has_debug_console() {
            let bytes = s.as_bytes();
            let slice = Physical::new(bytes.len(), bytes.as_ptr() as usize, 0);
            match sbi_rt::console_write(slice).into_result() {
                Ok(_) => Ok(()),
                Err(_) => Err(()),
            }
        } else {
            for b in s.bytes() {
                loop {
                    if let Ok(()) = RiscvSbiConsole::putchar(b) {
                        break;
                    }
                }
            }
            Ok(())
        }
    }
}
