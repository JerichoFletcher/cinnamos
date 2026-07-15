use core::{mem::MaybeUninit, num::NonZero, ptr::NonNull};

use uart::*;

use crate::arch;

struct SendUart(Uart<address::MmioAddress, Data>);

unsafe impl Sync for SendUart {}

static mut UART: MaybeUninit<SendUart> = MaybeUninit::zeroed();

pub fn init(base_addr: NonNull<u8>, irq_id: u16) {
    let mut drv = unsafe { <Uart<_, Data>>::new(address::MmioAddress::new(base_addr, 1)) };
    drv.write_fifo_control(
        FifoControl::ENABLE
            | FifoControl::INT_LVL_1
            | FifoControl::CLEAR_TX
            | FifoControl::CLEAR_RX,
    );

    if let Some(irq_id) = NonZero::new(irq_id) {
        drv.write_interrupt_enable(InterruptEnable::RECEIVED_DATA);
        let _ = arch::register_irq_handler(irq_id, handle_uart_irq);
    }
    unsafe {
        (&raw mut (UART)).write(MaybeUninit::new(SendUart(drv)));
    }
}

fn handle_uart_irq() {
    let drv = unsafe { &mut (&raw mut (UART)).as_mut_unchecked().assume_init_mut().0 };
    while drv.read_line_status().contains(LineStatus::DATA_AVAILABLE) {
        let b = drv.read_byte();

        // TODO: Push byte to input queue
        drv.write_byte(b);
    }
}

pub struct Writer;

impl Writer {
    pub fn new() -> Self {
        Self
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let drv = unsafe { &mut (&raw mut (UART)).as_mut_unchecked().assume_init_mut().0 };
        for c in s.bytes() {
            if c == b'\n' {
                drv.write_byte(b'\r');
                drv.write_byte(b'\n');
            } else {
                drv.write_byte(c);
            }
        }
        Ok(())
    }
}
