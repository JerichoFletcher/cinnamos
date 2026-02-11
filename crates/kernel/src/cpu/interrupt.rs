pub enum InterruptType {
    Timer,
    External,
    Software,
}

#[derive(PartialEq)]
pub struct InterruptMask {
    pub timer: bool,
    pub external: bool,
    pub software: bool,
}

impl Default for InterruptMask {
    fn default() -> Self {
        InterruptMask {
            timer: true,
            external: false,
            software: false,
        }
    }
}

pub struct InterruptController {
    mask: InterruptMask,
}

impl InterruptController {
    pub const fn new() -> Self {
        Self {
            mask: InterruptMask {
                timer: true,
                external: false,
                software: false,
            }
        }
    }

    pub fn enable(&mut self, interrupt: InterruptType) {
        match interrupt {
            InterruptType::Timer => self.mask.timer = true,
            InterruptType::External => self.mask.external = true,
            InterruptType::Software => self.mask.software = true,
        }
        self.sync();
    }

    pub fn disable(&mut self, interrupt: InterruptType) {
        match interrupt {
            InterruptType::Timer => self.mask.timer = false,
            InterruptType::External => self.mask.external = false,
            InterruptType::Software => self.mask.software = false,
        }
        self.sync();
    }

    fn sync(&self) {
        crate::arch::cpu::set_interrupt_mask(&self.mask);
    }
}


