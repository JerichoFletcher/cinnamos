use core::arch::asm;

#[inline(always)]
fn ecall(
    eid: usize,
    fid: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize
) -> Result<usize, usize> {
    let err: usize;
    let ret: usize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") a0 => err,
            inlateout("a1") a1 => ret,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
            in("a6") fid,
            in("a7") eid,
            options(nomem, nostack)
        );
    }
    if err == 0 {
        Ok(ret)
    } else {
        Err(err)
    }
}

#[inline(always)]
pub fn console_putchar(c: u8) {
    ecall(0x1, 0, c as usize, 0, 0, 0, 0, 0).expect("console_putchar failed");
}

#[inline(always)]
pub fn console_getchar() -> u8 {
    ecall(0x2, 0, 0, 0, 0, 0, 0, 0).expect("console_getchar failed") as u8
}
