.include "src/asm/common.s"

.section .text.boot.init
.global _start
_start:
    csrw    satp, zero
    la      t0, _boot_trap_vector
    csrw    stvec, t0

    la      sp, _boot_stack_top
    la      t0, _boot_data_start
    la      t1, _boot_data_end
1:
    bgeu    t0, t1, 2f
    sw      zero, 0(t0)
    addi    t0, t0, 4
    j       1b
2:
    addi    sp, sp, -16
    sw      ra, 12(sp)
    sw      a0, 8(sp)
    sw      a1, 4(sp)

    call    kinit_pt
    srli    t2, a0, 12
    li      t3, (1 << 31)
    or      t2, t2, t3
    csrw    satp, t2
    sfence.vma

    lw      a1, 4(sp)
    lw      a0, 8(sp)
    lw      ra, 12(sp)
    addi    sp, sp, 16
    tail    kentry

.section .text.boot
.align 2
_boot_trap_vector:
    addi    sp, sp, -16
    sw      t0, 12(sp)

    csrr    t0, scause
    addi    t0, t0, -3
    beqz    t0, 2f
1:
    wfi
    j       1b
2:
    csrr    t0, sepc
    addi    t0, t0, 4
    csrw    sepc, t0
    
    lw      t0, 12(sp)
    addi    sp, sp, 16
    sret

.section .text
kentry:
    la      sp, _kernel_stack_top
    la      t0, _bss_start
    la      t1, _bss_end
1:
    bgeu    t0, t1, 2f
    sw      zero, 0(t0)
    addi    t0, t0, 4
    j       1b
2:
    tail    kmain

.equ OFFSET_KERNEL_SP, 4
.equ OFFSET_SCRATCH, 8

.section .text
.align 2
.global _trap_vector
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
