use core::{num::NonZero, ptr::NonNull};

use uart::*;

use crate::{arch, console::ConsoleWrite, sync::mutex_irqsave::MutexIrqSave};

struct SendUart(Uart<address::MmioAddress, Data>);

static UART: MutexIrqSave<Option<SendUart>> = MutexIrqSave::new(None);

pub fn init(base_addr: NonNull<u8>, irq_id: u16) {
    let mut drv = unsafe { <Uart<_, Data>>::new(address::MmioAddress::new(base_addr, 1)) };
    drv.write_fifo_control(
        FifoControl::ENABLE
            | FifoControl::INT_LVL_1
            | FifoControl::CLEAR_TX
            | FifoControl::CLEAR_RX,
    );

    if let Some(irq_id) = NonZero::new(irq_id)
        && arch::register_irq_handler(irq_id, handle_uart_irq).is_ok()
    {
        drv.write_interrupt_enable(InterruptEnable::RECEIVED_DATA);
    }
    *UART.lock() = Some(SendUart(drv));
}

fn handle_uart_irq() {
    let mut g = UART.lock();
    if let Some(drv) = g.as_mut() {
        while drv
            .0
            .read_line_status()
            .contains(LineStatus::DATA_AVAILABLE)
        {
            let b = drv.0.read_byte();

            // TODO: Push byte to input queue
            drv.0.write_byte(b);
        }
    }
}

pub struct SerialWrite;

impl SerialWrite {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SerialWrite {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Write for SerialWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut g = UART.lock();
        if let Some(drv) = g.as_mut() {
            for c in s.bytes() {
                if c == b'\n' {
                    drv.0.write_byte(b'\r');
                    drv.0.write_byte(b'\n');
                } else {
                    drv.0.write_byte(c);
                }
            }
        }
        Ok(())
    }
}

impl ConsoleWrite for SerialWrite {
    fn flush(&mut self) {}
}
