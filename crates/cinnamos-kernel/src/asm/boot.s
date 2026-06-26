.section .text.init
.global _start
_start:
    la      t0, _bss_start
    la      t1, _bss_end
    bgeu    t0, t1, 2f

1:
    sd      zero, (t0)
    addi    t0, t0, 8
    bltu    t0, t1, 1b

2:
    la      sp, _stack_end
    la      t0, _trap_stack_end
    csrw    sscratch, t0
    tail    main
