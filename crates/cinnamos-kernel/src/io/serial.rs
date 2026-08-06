use structs::queue::BoundedQueue;

pub struct SerialInputBuffer {
    queue: BoundedQueue<u8, 128>,
}

impl SerialInputBuffer {
    const fn new() -> Self {
        Self { queue: BoundedQueue::new() }
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
impl super::Write for SerialInputWrite {
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
impl super::Read for SerialInputRead {
    type Error = ();

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut i = 0;
        while i < buf.len() && let Some(b) = SERIAL_INPUT_BUF.read() {
            buf[i] = b;
            i += 1;
        }
        Ok(i)
    }
}
