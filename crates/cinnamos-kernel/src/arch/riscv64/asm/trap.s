.equ HLOC_SCRATCH_OFFSET,   8
.equ HLOC_TASKPTR_OFFSET,   16
.equ HLOC_TSP_OFFSET,       24
.equ TASK_KSP_OFFSET,       8
.equ TRAP_FRAME_SIZE,       32*8 + 4*8

.section .text
.align 2
.global __trap_entry
__trap_entry:
# Load hart-local pointer from sscratch
# Load kernel stack pointer from current task
    csrrw   tp, sscratch, tp
    beq     tp, zero, __trap_entry_err_null_tp

    sd      sp, HLOC_SCRATCH_OFFSET(tp)
    ld      sp, HLOC_TASKPTR_OFFSET(tp)
    beq     sp, zero, 1f

# Load the task KSP
    ld      sp, TASK_KSP_OFFSET(sp)
    beq     sp, zero, __trap_entry_err_null_task_ksp
    j       2f

1:
# Null task pointer means the trap is taken before the scheduler starts
# Load the hart's trap SP instead
    ld      sp, HLOC_TSP_OFFSET(tp)
    beq     sp, zero, __trap_entry_err_null_kernel_tsp

2:
# Save registers (except tp and sp)
    addi    sp, sp, -TRAP_FRAME_SIZE
    sd      ra, 1*8(sp)
    sd      gp, 3*8(sp)
    sd      x5, 5*8(sp)
    sd      x6, 6*8(sp)
    sd      x7, 7*8(sp)
    sd      x8, 8*8(sp)
    sd      x9, 9*8(sp)
    sd      x10, 10*8(sp)
    sd      x11, 11*8(sp)
    sd      x12, 12*8(sp)
    sd      x13, 13*8(sp)
    sd      x14, 14*8(sp)
    sd      x15, 15*8(sp)
    sd      x16, 16*8(sp)
    sd      x17, 17*8(sp)
    sd      x18, 18*8(sp)
    sd      x19, 19*8(sp)
    sd      x20, 20*8(sp)
    sd      x21, 21*8(sp)
    sd      x22, 22*8(sp)
    sd      x23, 23*8(sp)
    sd      x24, 24*8(sp)
    sd      x25, 25*8(sp)
    sd      x26, 26*8(sp)
    sd      x27, 27*8(sp)
    sd      x28, 28*8(sp)
    sd      x29, 29*8(sp)
    sd      x30, 30*8(sp)
    sd      x31, 31*8(sp)

# Save task tp, sp, CSRs, and trap information
    ld      t0, HLOC_SCRATCH_OFFSET(tp)
    csrr    t1, sscratch
    csrr    t2, sstatus
    csrr    t3, sepc
    csrr    t4, scause
    csrr    t5, stval

    sd      t0, 2*8(sp)
    sd      t1, 4*8(sp)
    sd      t2, 32*8(sp)
    sd      t3, 33*8(sp)
    sd      t4, 34*8(sp)
    sd      t5, 35*8(sp)

# Restore hart-local pointer to sscratch
    csrw    sscratch, tp

# Call Rust handler
    mv      a0, sp
    mv      a1, tp
    call    trap_handler
    j       __trap_exit

.global __trap_exit
__trap_exit:
    addi    t0, sp, TRAP_FRAME_SIZE
    ld      t1, HLOC_TASKPTR_OFFSET(tp)
    beq     t1, zero, 1f

# Store new task KSP ahead of time
    sd      t0, TASK_KSP_OFFSET(t1)
    j       2f

1:
# Store new kernel TSP ahead of time
# Shouldn't matter but will make improper stack switches visible
    sd      t0, HLOC_TSP_OFFSET(tp)

2:
# Restore CSRs
    ld      t0, 2*8(sp)
    ld      t1, 4*8(sp)
    ld      t2, 32*8(sp)
    ld      t3, 33*8(sp)

    sd      t0, HLOC_SCRATCH_OFFSET(tp)
    csrw    sscratch, t1
    csrw    sstatus, t2
    csrw    sepc, t3

# Restore registers (except sp)
    ld      ra, 1*8(sp)
    ld      gp, 3*8(sp)
    ld      x5, 5*8(sp)
    ld      x6, 6*8(sp)
    ld      x7, 7*8(sp)
    ld      x8, 8*8(sp)
    ld      x9, 9*8(sp)
    ld      x10, 10*8(sp)
    ld      x11, 11*8(sp)
    ld      x12, 12*8(sp)
    ld      x13, 13*8(sp)
    ld      x14, 14*8(sp)
    ld      x15, 15*8(sp)
    ld      x16, 16*8(sp)
    ld      x17, 17*8(sp)
    ld      x18, 18*8(sp)
    ld      x19, 19*8(sp)
    ld      x20, 20*8(sp)
    ld      x21, 21*8(sp)
    ld      x22, 22*8(sp)
    ld      x23, 23*8(sp)
    ld      x24, 24*8(sp)
    ld      x25, 25*8(sp)
    ld      x26, 26*8(sp)
    ld      x27, 27*8(sp)
    ld      x28, 28*8(sp)
    ld      x29, 29*8(sp)
    ld      x30, 30*8(sp)
    ld      x31, 31*8(sp)

# Restore context stack pointer
    addi    sp, sp, TRAP_FRAME_SIZE
    ld      sp, HLOC_SCRATCH_OFFSET(tp)
    csrrw   tp, sscratch, tp
    sret

# Null TP should never happen since we initialized hart-locals early
__trap_entry_err_null_tp:
1:
    j       1b

# Tasks should never be created with a null KSP
__trap_entry_err_null_task_ksp:
1:
    j       1b

# Initialized hart-locals should never have a null TSP
__trap_entry_err_null_kernel_tsp:
1:
    j       1b
