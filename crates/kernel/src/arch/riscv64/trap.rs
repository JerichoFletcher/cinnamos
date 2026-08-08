use cinnamos_abi::{Syscall, SyscallError};
use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        sscratch,
        sstatus::{self, Sstatus},
        stval,
        stvec::{self, Stvec, TrapMode},
    },
};

use crate::{
    arch::{self, Context, VAddr, interrupt},
    *,
};

const _: () = debug_assert!(size_of::<TrapFrame>() == (34 * size_of::<usize>()));

unsafe extern "C" {
    /// Loads the kernel stack pointer from the current task if it exists (or the hart's global stack),
    /// pushes a [TrapFrame], and calls [trap_handler].
    fn __trap_entry() -> !;
    /// Pops a [TrapFrame], saves the new kernel stack pointer to the current task if it exists,
    /// and exits the trap.
    fn __trap_exit() -> !;
}

/// A frame containing the snapshot of the hart state.
#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    /// The contents of all registers in the snapshot. Allows accessing and modifying the register
    /// values for the corresponding hart state.
    pub regs: [usize; 32],
    /// The value of the `sstatus` CSR. Allows accessing and modifying the value of `sstatus`
    /// for the corresponding hart state.
    pub sstatus: Sstatus,
    /// The value of the `sepc` CSR, which is the address that the trap handler will return
    /// to after exiting the trap.
    pub sepc: VAddr,
}

impl TrapFrame {
    pub const REG_ZERO: usize = 0;
    pub const REG_RA: usize = 1;
    pub const REG_SP: usize = 2;
    pub const REG_GP: usize = 3;
    pub const REG_TP: usize = 4;
    pub const REG_T0: usize = 5;
    pub const REG_T1: usize = 6;
    pub const REG_T2: usize = 7;
    pub const REG_S0: usize = 8;
    pub const REG_S1: usize = 9;
    pub const REG_A0: usize = 10;
    pub const REG_A1: usize = 11;
    pub const REG_A2: usize = 12;
    pub const REG_A3: usize = 13;
    pub const REG_A4: usize = 14;
    pub const REG_A5: usize = 15;
    pub const REG_A6: usize = 16;
    pub const REG_A7: usize = 17;
    pub const REG_S2: usize = 18;
    pub const REG_S3: usize = 19;
    pub const REG_S4: usize = 20;
    pub const REG_S5: usize = 21;
    pub const REG_S6: usize = 22;
    pub const REG_S7: usize = 23;
    pub const REG_S8: usize = 24;
    pub const REG_S9: usize = 25;
    pub const REG_S10: usize = 26;
    pub const REG_S11: usize = 27;
    pub const REG_T3: usize = 28;
    pub const REG_T4: usize = 29;
    pub const REG_T5: usize = 30;
    pub const REG_T6: usize = 31;

    fn create_kernel_frame(entry: *const (), stack_ptr: VAddr) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spie(true);
        sstatus.set_spp(sstatus::SPP::Supervisor);

        let mut regs = [0; 32];
        regs[Self::REG_SP] = stack_ptr.addr();
        regs[Self::REG_TP] = hloc::get_ptr() as usize;
        Self {
            regs,
            sstatus,
            sepc: VAddr::from_ptr(entry),
        }
    }
}

/// Fabricates a snapshot of an empty hart state with the given stack pointer. When loading this frame,
/// the hart will load and jump to `entry`.
///
/// # Safety
/// - `entry` must point to executable code (e.g. a function or user task entry point).
/// - `task_sp` must point to a valid stack memory.
pub unsafe fn create_init_trap_frame(entry: *const (), task_sp: VAddr) -> TrapFrame {
    TrapFrame::create_kernel_frame(entry, task_sp)
}

/// Fabricates an initial context that simply jumps to trap exit. Intended to be coupled with a [TrapFrame]
/// to form a complete stack that loads the frame and jumps to its entry point.
pub fn create_init_context() -> Context {
    Context::new(VAddr::from_ptr(__trap_exit as *const ()))
}

/// Dispatches the appropriate handler for a trap.
#[unsafe(no_mangle)]
extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let tcause = riscv::interrupt::cause();
    let tval = stval::read();

    match tcause {
        Trap::Exception(Exception::InstructionMisaligned) => panic!(
            "[at {:#016x}] Instruction misaligned {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::InstructionFault) => panic!(
            "[at {:#016x}] Instruction fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::IllegalInstruction) => panic!(
            "[at {:#016x}] Illegal instruction {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::LoadMisaligned) => panic!(
            "[at {:#016x}] Load misaligned {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::LoadFault) => panic!(
            "[at {:#016x}] Load fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::StoreMisaligned) => panic!(
            "[at {:#016x}] Store misaligned {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::StoreFault) => panic!(
            "[at {:#016x}] Store fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::InstructionPageFault) => panic!(
            "[at {:#016x}] Instruction page fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::LoadPageFault) => panic!(
            "[at {:#016x}] Load page fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::StorePageFault) => panic!(
            "[at {:#016x}] Store page fault {:#016x}",
            frame.sepc, tval
        ),
        Trap::Exception(Exception::UserEnvCall) => {
            // Safety: Syscalls are invoked from the userspace using the call stubs generated from
            // the required argument types, and as such the argument values within the frame are
            // valid for their respective types
            unsafe { dispatch_syscall(frame) };
            frame.sepc = frame.sepc + 4;
        }
        Trap::Exception(Exception::Breakpoint) => {
            frame.sepc = frame.sepc + 4;
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            arch::timer::schedule_timer();
            hloc::try_with_critical(|mut hloc| {
                if let Some(curr) = hloc.curr_task() {
                    let t = curr.tcb().time_quantum;
                    if t == 0 {
                        sched::schedule();
                    } else {
                        curr.tcb_mut().time_quantum -= 1;
                    }
                }
            })
            .expect("failed to get hart-local storage");
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            handle_external_interrupt();
        }
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            log::trace!(
                "[at {:#016x}] Software interrupt {:#016x}",
                frame.sepc, tval
            );
        }
        _ => panic!(
            "[at {:#016x}] Unhandled trap {:?} {:#016x}",
            frame.sepc, tcause, tval
        ),
    }
}

/// Dispatches a system call that was requested in the frame.
///
/// # Safety
/// `frame` must contain valid bit patterns for the argument types of the system call being invoked.
unsafe fn dispatch_syscall(frame: &mut TrapFrame) {
    match Syscall::try_from(frame.regs[TrapFrame::REG_A7]) {
        Ok(sys) => {
            if let Some(args) = frame.regs[TrapFrame::REG_A0..TrapFrame::REG_A5].first_chunk::<6>() {
                // Safety: args passed from the context are valid for their argument types
                match unsafe { sys::dispatch_syscall(sys, args) } {
                    Ok(ret) => {
                        log::trace!("syscall OK sys={:?} ret={}", sys, ret);
                        frame.regs[TrapFrame::REG_A0] = 0;
                        frame.regs[TrapFrame::REG_A1] = ret;
                    }
                    Err(e) => {
                        log::trace!("syscall ERR sys={:?} err={:?}", sys, e);
                        frame.regs[TrapFrame::REG_A0] = e.into();
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("syscall ERR sys={} unknown", e.number);
            frame.regs[TrapFrame::REG_A0] = SyscallError::UnknownSyscall.into();
        }
    }
}

/// Attempts to claim an IRQ and dispatch the appropriate handler. It is possible and allowed that
/// no enabled IRQs are pending, in which case this function will do nothing.
fn handle_external_interrupt() {
    let hid = hloc::get_hid();
    // Safety: The ID is read from the current hart-local
    if let Some(claim) = unsafe { arch::device::plic::claim_irq(hid) } {
        let irq = claim.irq_id();
        if let Err(e) = interrupt::dispatch_irq(irq) {
            log::warn!(
                "failed to handle claimed interrupt hid={} irq={}: {:?}",
                hid,
                irq,
                e
            );
        }
    }
}

/// Installs the trap vector and loads the hart-local pointer onto `sscratch`.
pub fn init() {
    let trap_entry_addr = __trap_entry as *const () as usize;
    let stvec = Stvec::new(trap_entry_addr, TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
        sscratch::write(hloc::get_ptr() as usize);
    }
}
