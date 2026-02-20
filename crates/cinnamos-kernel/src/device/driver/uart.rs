use core::fmt::Write;
use bitflags::bitflags;
use generic_once_cell::OnceCell;
use crate::lock::RawSpinLock;

#[repr(usize)]
pub enum ReadRegister {
    ReceiverBuffer = 0,
    InterruptEnable = 1,
    InterruptIdent = 2,
    LineControl = 3,
    ModemControl = 4,
    LineStatus = 5,
    ModemStatus = 6,
    Scratch = 7,
}

impl ReadRegister {
    // const DIVISOR_ACCESS_LO: Self = Self::ReceiverBuffer;
    // const DIVISOR_ACCESS_HI: Self = Self::InterruptEnable;

    #[inline]
    pub fn val(self) -> usize {
        self as usize
    }
}

#[repr(usize)]
pub enum WriteRegister {
    TransmitterBuffer = 0,
    InterruptEnable = 1,
    FifoControl = 2,
    LineControl = 3,
    ModemControl = 4,
    LineStatus = 5,
    ModemStatus = 6,
    Scratch = 7,
}

impl WriteRegister {
    const DIVISOR_ACCESS_LO: Self = Self::TransmitterBuffer;
    const DIVISOR_ACCESS_HI: Self = Self::InterruptEnable;

    #[inline]
    pub fn val(self) -> usize {
        self as usize
    }
}

bitflags! {
    pub struct InterruptEnable : u8 {
        const ReceivedDataAvailable = 1 << 0;
        const TransmitterHoldingRegisterEmpty = 1 << 1;
        const ReceiverLineStatus = 1 << 2;
        const ModemStatus = 1 << 3;
    }
}

bitflags! {
    pub struct FifoControl : u8 {
        const Enable = 1 << 0;
        const ResetRcvr = 1 << 1;
        const ResetXmit = 1 << 2;
    }
}

bitflags! {
    pub struct LineControl : u8 {
        const Word5Bits = 0b00;
        const Word6Bits = 0b01;
        const Word7Bits = 0b10;
        const Word8Bits = 0b11;

        const Stop1Bit = 0 << 2;
        const Stop2Bits = 1 << 2;

        const ParityEnabled = 1 << 3;
        const ParityOdd = 0 << 4;
        const ParityEven = 1 << 4;
        const ParityForce = 1 << 5;

        const SetBreak = 1 << 6;
        const DivisorLatchAccess = 1 << 7;
    }
}

bitflags! {
    pub struct LineStatus : u8 {
        const DataReady = 1 << 0;
        const OverrunError = 1 << 1;
        const ParityError = 1 << 2;
        const FramingError = 1 << 3;
        const BreakInterrupt = 1 << 4;
        const TransmitterHolding = 1 << 5;
        const TransmitterEmpty = 1 << 6;
        const ReceiverFifoError = 1 << 7;
    }
}

static DRIVER: OnceCell<RawSpinLock, Uart> = OnceCell::new();

pub struct Uart {
    addr: usize,
}

impl Uart {
    pub const BAUD_SPS: usize = 2400;

    pub fn global<'a>() -> &'a Uart {
        DRIVER.get().expect("UART driver not initialized")
    }

    pub fn init<'a>(addr: usize) -> &'a Uart {
        DRIVER.get_or_init(|| Uart::new(addr))
    }

    pub const fn base_address(&self) -> usize {
        self.addr
    }

    pub fn adjust_to_clock_freq(&self, clk_freq: usize) {
        let mut div = clk_freq / (16 * Self::BAUD_SPS);
        let rem = clk_freq % (16 * Self::BAUD_SPS);
        if rem != 0 {
            div += 1
        }

        let div = div as u16;
        let div_lo = (div & 0xff) as u8;
        let div_hi = ((div >> 8) & 0xff) as u8;

        self.set_divisor_latch_access(true);
        self.write_reg(WriteRegister::DIVISOR_ACCESS_LO, div_lo);
        self.write_reg(WriteRegister::DIVISOR_ACCESS_HI, div_hi);
        self.set_divisor_latch_access(false);
    }

    pub fn put(&self, c: u8) {
        self.write_reg(WriteRegister::TransmitterBuffer, c);
    }

    pub fn get(&self) -> Option<u8> {
        if self.line_status().contains(LineStatus::DataReady) {
            Some(self.read_reg(ReadRegister::ReceiverBuffer))
        } else {
            None
        }
    }

    fn new(addr: usize) -> Self {
        let drv = Self { addr };

        // Word length = 8 bits
        drv.set_line_control(LineControl::Word8Bits);
        // Enable FIFO
        drv.write_reg(WriteRegister::FifoControl, FifoControl::Enable.bits());
        // Enable receiver buffer interrupt
        drv.write_reg(WriteRegister::InterruptEnable, InterruptEnable::ReceivedDataAvailable.bits());

        drv
    }

    fn set_divisor_latch_access(&self, access: bool) {
        let mut flags = self.line_control();
        flags.set(LineControl::DivisorLatchAccess, access);
        self.set_line_control(flags);
    }

    #[inline]
    fn line_control(&self) -> LineControl {
        LineControl::from_bits(self.read_reg(ReadRegister::LineControl)).unwrap()
    }

    #[inline]
    fn set_line_control(&self, flags: LineControl) {
        self.write_reg(WriteRegister::LineControl, flags.bits());
    }

    #[inline]
    fn line_status(&self) -> LineStatus {
        LineStatus::from_bits(self.read_reg(ReadRegister::LineStatus)).unwrap()
    }

    #[inline]
    fn write_reg(&self, reg: WriteRegister, bits: u8) {
        let ptr = self.addr as *mut u8;
        unsafe { ptr.add(reg.val()).write_volatile(bits); }
    }

    #[inline]
    fn read_reg(&self, reg: ReadRegister) -> u8 {
        let ptr = self.addr as *mut u8;
        unsafe { ptr.add(reg.val()).read_volatile() }
    }
}

pub struct UartWrite;

impl UartWrite {
    pub fn new() -> Self {
        Self {}
    }
}

impl Write for UartWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let drv = Uart::global();
        for c in s.bytes() {
            drv.put(c);
        }
        Ok(())
    }
}
