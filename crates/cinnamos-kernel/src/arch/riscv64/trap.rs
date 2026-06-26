use riscv::{interrupt::{Exception, Interrupt, Trap}, register::{scause::Scause, stvec::{self, Stvec, TrapMode}}};

use crate::{arch::{self, context::RiscvContext}, println};

#[repr(C)]
struct RiscvTrapFrame {
    ctx: RiscvContext,
    scause: Scause,
    stval: usize,
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut RiscvTrapFrame) {
    let tcause = frame.scause.cause().try_into::<Interrupt, Exception>().unwrap();
    let sepc = frame.ctx.sepc as usize;

    match tcause {
        Trap::Exception(Exception::IllegalInstruction) => panic!("[at 0x{:016x}] Illegal instruction 0x{:016x}", sepc, frame.stval),
        Trap::Exception(Exception::InstructionMisaligned)
        | Trap::Exception(Exception::LoadMisaligned)
        | Trap::Exception(Exception::StoreMisaligned) => panic!("[at 0x{:016x}] Access misaligned 0x{:016x}", sepc, frame.stval),
        Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::StorePageFault) => panic!("[at 0x{:016x}] Access page fault 0x{:016x}", sepc, frame.stval),
        Trap::Exception(Exception::UserEnvCall) => {
            println!("[at 0x{:016x}] U-mode syscall {}", sepc, frame.ctx.regs[17]);
            frame.ctx.sepc = unsafe { frame.ctx.sepc.byte_add(4) };
        },
        Trap::Exception(Exception::Breakpoint) => {
            frame.ctx.sepc = unsafe { frame.ctx.sepc.byte_add(4) };
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            arch::timer::schedule_timer();
        },
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            println!("[at 0x{:016x}] External interrupt", sepc);
        },
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            println!("[at 0x{:016x}] Software interrupt", sepc);
        },
        _ => (),
    }
}

pub fn init() {
    unsafe extern "C" {
        fn _trap_entry();
    }
    let trap_entry_addr = _trap_entry as *const() as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe { stvec::write(stvec); }
}
