cfg_select! {
    target_arch = "riscv64" => {
        use crate::arch::riscv64;
        pub use riscv64::ic::{
            InterruptController, InterruptPriority, InterruptPriorityThreshold, InterruptSource,
        };
        pub use riscv64::interrupt::{
            InterruptError, IrqDisabledSection, MasksIrq, interrupt_free, interrupt_nested,
            register_irq_handler,
        };
        pub use riscv64::{init_interrupt_driver as init_driver, init_interrupts as init};
    }
}
