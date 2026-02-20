.section .text.boot
.global KERNEL_VMA
KERNEL_VMA:         .dword _kernel_vma
.global KERNEL_LMA
KERNEL_LMA:         .dword _kernel_lma
.global BOOT_START
BOOT_START:         .dword _boot_start
.global BOOT_END
BOOT_END:           .dword _boot_end
.global KERNEL_START
KERNEL_START:       .dword _kernel_start
.global KERNEL_END
KERNEL_END:         .dword _kernel_end
.global KERNEL_PHYS_START
KERNEL_PHYS_START:  .dword _kernel_phys_start
.global KERNEL_PHYS_END
KERNEL_PHYS_END:    .dword _kernel_phys_end
