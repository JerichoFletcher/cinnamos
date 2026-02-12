.global _trap_vector

.equ OFFSET_KERNEL_SP, 4
.equ OFFSET_SCRATCH, 8

.section .text
.align 2
_trap_vector:
    # Load kernel context ptr from sscratch
    # Init trap stack frame
    csrrw   tp, sscratch, tp
    sw      sp, OFFSET_SCRATCH(tp)
    lw      sp, OFFSET_KERNEL_SP(tp)
    addi    sp, sp, -144

    # Save registers (except sp, tp)
    sw      ra, 4(sp)
    sw      gp, 12(sp)
    sw      x5, 20(sp)
    sw      x6, 24(sp)
    sw      x7, 28(sp)
    sw      x8, 32(sp)
    sw      x9, 36(sp)
    sw      x10, 40(sp)
    sw      x11, 44(sp)
    sw      x12, 48(sp)
    sw      x13, 52(sp)
    sw      x14, 56(sp)
    sw      x15, 60(sp)
    sw      x16, 64(sp)
    sw      x17, 68(sp)
    sw      x18, 72(sp)
    sw      x19, 76(sp)
    sw      x20, 80(sp)
    sw      x21, 84(sp)
    sw      x22, 88(sp)
    sw      x23, 92(sp)
    sw      x24, 96(sp)
    sw      x25, 100(sp)
    sw      x26, 104(sp)
    sw      x27, 108(sp)
    sw      x28, 112(sp)
    sw      x29, 116(sp)
    sw      x30, 120(sp)
    sw      x31, 124(sp)

    lw      t0, OFFSET_SCRATCH(tp)
    csrr    t1, sscratch
    csrr    t2, sstatus
    csrr    t3, sepc
    csrr    t4, scause
    csrr    t5, stval

    sw      t0, 8(sp)
    sw      t1, 16(sp)
    sw      t2, 128(sp)
    sw      t3, 132(sp)
    sw      t4, 136(sp)
    sw      t5, 140(sp)
    # State:
    # sscratch = user tp
    # OFFSET_SCRATCH(tp) = user sp

    # Pass trap frame pointer
    mv      a0, sp
    call    trap_handler

    # Expected:
    # sscratch <- new user tp
    # OFFSET_SCRATCH(tp) <- new user sp
    lw      t0, 8(sp)
    lw      t1, 16(sp)
    lw      t2, 128(sp)
    lw      t3, 132(sp)

    sw      t0, OFFSET_SCRATCH(tp)
    csrw    sscratch, t1
    csrw    sstatus, t2
    csrw    sepc, t3

    # Restore registers (except sp)
    lw      ra, 4(sp)
    lw      gp, 12(sp)
    lw      x5, 20(sp)
    lw      x6, 24(sp)
    lw      x7, 28(sp)
    lw      x8, 32(sp)
    lw      x9, 36(sp)
    lw      x10, 40(sp)
    lw      x11, 44(sp)
    lw      x12, 48(sp)
    lw      x13, 52(sp)
    lw      x14, 56(sp)
    lw      x15, 60(sp)
    lw      x16, 64(sp)
    lw      x17, 68(sp)
    lw      x18, 72(sp)
    lw      x19, 76(sp)
    lw      x20, 80(sp)
    lw      x21, 84(sp)
    lw      x22, 88(sp)
    lw      x23, 92(sp)
    lw      x24, 96(sp)
    lw      x25, 100(sp)
    lw      x26, 104(sp)
    lw      x27, 108(sp)
    lw      x28, 112(sp)
    lw      x29, 116(sp)
    lw      x30, 120(sp)
    lw      x31, 124(sp)

    lw      sp, OFFSET_SCRATCH(sp)
    csrrw   tp, sscratch, tp
    sret
