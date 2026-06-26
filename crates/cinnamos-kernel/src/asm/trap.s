.section .text
.align 2
.global _trap_entry
_trap_entry:
# Load trap handler stack pointer from sscratch
    csrrw   sp, sscratch, sp
    addi    sp, sp, -288

# Save registers (except sp)
    sd      ra, 8(sp)
    sd      gp, 24(sp)
    sd      tp, 32(sp)
    sd      x5, 40(sp)
    sd      x6, 48(sp)
    sd      x7, 56(sp)
    sd      x8, 64(sp)
    sd      x9, 72(sp)
    sd      x10, 80(sp)
    sd      x11, 88(sp)
    sd      x12, 96(sp)
    sd      x13, 104(sp)
    sd      x14, 112(sp)
    sd      x15, 120(sp)
    sd      x16, 128(sp)
    sd      x17, 136(sp)
    sd      x18, 144(sp)
    sd      x19, 152(sp)
    sd      x20, 160(sp)
    sd      x21, 168(sp)
    sd      x22, 176(sp)
    sd      x23, 184(sp)
    sd      x24, 192(sp)
    sd      x25, 200(sp)
    sd      x26, 208(sp)
    sd      x27, 216(sp)
    sd      x28, 224(sp)
    sd      x29, 232(sp)
    sd      x30, 240(sp)
    sd      x31, 248(sp)

# Save task stack pointer, context, and trap information
    csrr    t0, sscratch
    csrr    t1, sstatus
    csrr    t2, sepc
    csrr    t3, scause
    csrr    t4, stval

    sd      t0, 16(sp)
    sd      t1, 256(sp)
    sd      t2, 264(sp)
    sd      t3, 272(sp)
    sd      t4, 280(sp)

# Call Rust handler
    mv      a0, sp
    call    trap_handler

# Restore new context
    ld      t0, 16(sp)
    ld      t1, 256(sp)
    ld      t2, 264(sp)

    csrw    sscratch, t0
    csrw    sstatus, t1
    csrw    sepc, t2

# Restore registers (except sp)
    ld      ra, 8(sp)
    ld      gp, 24(sp)
    ld      tp, 32(sp)
    ld      x5, 40(sp)
    ld      x6, 48(sp)
    ld      x7, 56(sp)
    ld      x8, 64(sp)
    ld      x9, 72(sp)
    ld      x10, 80(sp)
    ld      x11, 88(sp)
    ld      x12, 96(sp)
    ld      x13, 104(sp)
    ld      x14, 112(sp)
    ld      x15, 120(sp)
    ld      x16, 128(sp)
    ld      x17, 136(sp)
    ld      x18, 144(sp)
    ld      x19, 152(sp)
    ld      x20, 160(sp)
    ld      x21, 168(sp)
    ld      x22, 176(sp)
    ld      x23, 184(sp)
    ld      x24, 192(sp)
    ld      x25, 200(sp)
    ld      x26, 208(sp)
    ld      x27, 216(sp)
    ld      x28, 224(sp)
    ld      x29, 232(sp)
    ld      x30, 240(sp)
    ld      x31, 248(sp)

# Restore context stack pointer
    addi    sp, sp, 288
    csrrw   sp, sscratch, sp

    sret
