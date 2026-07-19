use riscv::{interrupt::{Exception, Interrupt, Trap}, register::{scause::Scause, sscratch, stvec::{self, Stvec, TrapMode}}};

use crate::{arch::{self, context::Context, interrupt}, hloc::HartLocal, *};

#[repr(C)]
struct TrapFrame {
    ctx: Context,
    scause: Scause,
    stval: usize,
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let tcause = frame.scause.cause().try_into::<Interrupt, Exception>().unwrap();

    match tcause {
        Trap::Exception(Exception::IllegalInstruction) => panic!("[at 0x{:016x}] Illegal instruction 0x{:016x}", frame.ctx.sepc, frame.stval),
        Trap::Exception(Exception::InstructionMisaligned)
        | Trap::Exception(Exception::LoadMisaligned)
        | Trap::Exception(Exception::StoreMisaligned) => panic!("[at 0x{:016x}] Access misaligned 0x{:016x}", frame.ctx.sepc, frame.stval),
        Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::StorePageFault) => panic!("[at 0x{:016x}] Access page fault 0x{:016x}", frame.ctx.sepc, frame.stval),
        Trap::Exception(Exception::UserEnvCall) => {
            println!("[at 0x{:016x}] U-mode syscall {}", frame.ctx.sepc, frame.ctx.regs[17]);
            frame.ctx.sepc = frame.ctx.sepc + 4;
        },
        Trap::Exception(Exception::Breakpoint) => {
            frame.ctx.sepc = frame.ctx.sepc + 4;
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            arch::timer::schedule_timer();
        },
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            handle_external_interrupt(frame);
        },
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            println!("[at 0x{:016x}] Software interrupt 0x{:016x}", frame.ctx.sepc, frame.stval);
        },
        _ => (),
    }
}

fn handle_external_interrupt(_frame: &mut TrapFrame) {
    let hloc = hloc::hart_local();
    if let Some(claim) = arch::device::plic::claim_irq(hloc.hid) {
        let irq = claim.irq_id();
        if let Err(e) = interrupt::dispatch_irq(irq) {
            println!("warn : (HID {}) failed to handle claimed interrupt {}: {:?}", hloc.hid, irq, e);
        }
    }
}

pub fn init() {
    unsafe extern "C" {
        fn _trap_entry();
    }

    let trap_entry_addr = _trap_entry as *const() as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::hart_local() as *const HartLocal as usize);
    }
}

pub fn init_higher_half() {
    unsafe extern "C" {
        fn _trap_entry();
    }

    let trap_entry_addr = _trap_entry as *const() as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::hart_local() as *const HartLocal as usize);
    }
}
