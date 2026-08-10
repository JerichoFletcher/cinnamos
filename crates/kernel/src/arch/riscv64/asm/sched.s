.equ SWITCH_FRAME_SIZE,     21*8
.equ TASK_CTXSP_OFFSET,     0

.section .text
.global __switch
# a0: *const Task   = ptr to next task
# a1: *mut Task     = ptr to curr task
__switch:
    addi    sp, sp, -SWITCH_FRAME_SIZE
    sd      ra, 0*8(sp)
    sd      s0, 1*8(sp)
    sd      s1, 2*8(sp)
    sd      s2, 3*8(sp)
    sd      s3, 4*8(sp)
    sd      s4, 5*8(sp)
    sd      s5, 6*8(sp)
    sd      s6, 7*8(sp)
    sd      s7, 8*8(sp)
    sd      s8, 9*8(sp)
    sd      s9, 10*8(sp)
    sd      s10, 11*8(sp)
    sd      s11, 12*8(sp)
    sd      a0, 13*8(sp)
    sd      a1, 14*8(sp)
    sd      a2, 15*8(sp)
    sd      a3, 16*8(sp)
    sd      a4, 17*8(sp)
    sd      a5, 18*8(sp)
    sd      a6, 19*8(sp)
    sd      a7, 20*8(sp)

    sd      sp, TASK_CTXSP_OFFSET(a1)
    j       __switch_noprev

.global __switch_noprev
# a0: *const Task   = ptr to next task
__switch_noprev:
    ld      sp, TASK_CTXSP_OFFSET(a0)

    ld      ra, 0*8(sp)
    ld      s0, 1*8(sp)
    ld      s1, 2*8(sp)
    ld      s2, 3*8(sp)
    ld      s3, 4*8(sp)
    ld      s4, 5*8(sp)
    ld      s5, 6*8(sp)
    ld      s6, 7*8(sp)
    ld      s7, 8*8(sp)
    ld      s8, 9*8(sp)
    ld      s9, 10*8(sp)
    ld      s10, 11*8(sp)
    ld      s11, 12*8(sp)
    ld      a0, 13*8(sp)
    ld      a1, 14*8(sp)
    ld      a2, 15*8(sp)
    ld      a3, 16*8(sp)
    ld      a4, 17*8(sp)
    ld      a5, 18*8(sp)
    ld      a6, 19*8(sp)
    ld      a7, 20*8(sp)
    addi    sp, sp, SWITCH_FRAME_SIZE
    ret
