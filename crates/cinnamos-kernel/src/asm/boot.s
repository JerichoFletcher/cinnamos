.section .text.init
.global _start
_start:
    lla     t0, _bss_start
    lla     t1, _bss_end
    bgeu    t0, t1, 2f

1:
    sd      zero, (t0)
    addi    t0, t0, 8
    bltu    t0, t1, 1b

2:
    lla     sp, _stack_end
    lla     a2, _DYNAMIC
    tail    kernel_relocate
