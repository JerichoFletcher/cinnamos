use core::arch::asm;

use crate::{Syscall, SyscallError};

#[inline(always)]
pub unsafe fn syscall0(sys: Syscall) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            out("a0") status,
            out("a1") retval,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall1(sys: Syscall, arg0: usize) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            out("a1") retval,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall2(sys: Syscall, arg0: usize, arg1: usize) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            inlateout("a1") arg1 => retval,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall3(
    sys: Syscall,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            inlateout("a1") arg1 => retval,
            in("a2") arg2,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall4(
    sys: Syscall,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            inlateout("a1") arg1 => retval,
            in("a2") arg2,
            in("a3") arg3,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall5(
    sys: Syscall,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            inlateout("a1") arg1 => retval,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}

#[inline(always)]
pub unsafe fn syscall6(
    sys: Syscall,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> Result<usize, SyscallError> {
    let sys: usize = sys.into();
    let status: usize;
    let retval: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => status,
            inlateout("a1") arg1 => retval,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a7") sys,
            options(nostack, preserves_flags),
        );
    }
    if status == 0 {
        Ok(retval)
    } else {
        Err(status.into())
    }
}
