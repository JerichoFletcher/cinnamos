use core::{fmt, num::NonZero, ptr::NonNull};

use structs::queue::BoundedQueue;
use uart::*;

use super::{Read, Write};
use crate::{
    arch,
    console::ConsoleWrite,
    device::uart::{UartReceiveRead, UartTransmitWrite},
    sync::mutex_irqsave::MutexIrqSave,
};

pub struct SerialInputBuffer {
    queue: BoundedQueue<u8, 128>,
}

impl SerialInputBuffer {
    const fn new() -> Self {
        Self {
            queue: BoundedQueue::new(),
        }
    }

    fn read(&self) -> Option<u8> {
        self.queue.dequeue()
    }

    /// Drops the byte if the queue is full.
    fn write(&self, byte: u8) -> bool {
        self.queue.enqueue(byte).is_ok()
    }
}

static SERIAL_INPUT_BUF: SerialInputBuffer = SerialInputBuffer::new();

pub struct SerialInputWrite;
impl Write for SerialInputWrite {
    type Error = ();

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut i = 0;
        while i < buf.len() && SERIAL_INPUT_BUF.write(buf[i]) {
            i += 1;
        }
        Ok(i)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct SerialInputRead;
impl Read for SerialInputRead {
    type Error = ();

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut i = 0;
        while i < buf.len()
            && let Some(b) = SERIAL_INPUT_BUF.read()
        {
            buf[i] = b;
            i += 1;
        }
        Ok(i)
    }
}

struct SendUart(Uart<address::MmioAddress, Data>);

static IO_UART: MutexIrqSave<Option<SendUart>> = MutexIrqSave::new(None);

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
    *IO_UART.lock() = Some(SendUart(drv));
}

fn handle_uart_irq() {
    let mut g = IO_UART.lock();
    if let Some(drv) = g.as_mut() {
        let mut buf = [0u8; 32];
        let mut reader = UartReceiveRead::new(&mut drv.0);

        while let Ok(len) = reader.read(&mut buf)
            && len > 0
        {
            let _ = SerialInputWrite.write(&buf[..len]);
        }
    }
}

pub struct SerialOutputWrite;
impl fmt::Write for SerialOutputWrite {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut g = IO_UART.lock();
        if let Some(drv) = g.as_mut() {
            let mut writer = UartTransmitWrite::new(&mut drv.0);
            let mut len = 0;
            while len < s.len() {
                match writer.write(&s.as_bytes()[len..]) {
                    Ok(chunk) => len += chunk,
                    Err(()) => return Err(fmt::Error),
                }
            }
        }
        Ok(())
    }
}

impl ConsoleWrite for SerialOutputWrite {
    fn flush(&mut self) {}
}
