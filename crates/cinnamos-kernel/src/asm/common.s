.section .text.boot
.global KERNEL_VMA
.global KERNEL_LMA
KERNEL_VMA:         .dword _kernel_vma
KERNEL_LMA:         .dword _kernel_lma

.global BOOT_START
.global BOOT_END
BOOT_START:         .dword _boot_start
BOOT_END:           .dword _boot_end

.global BOOT_TEXT_START
.global BOOT_TEXT_END
BOOT_TEXT_START:    .dword _boot_text_start
BOOT_TEXT_END:      .dword _boot_text_end

.global KERNEL_PHYS_START
.global KERNEL_PHYS_END
KERNEL_PHYS_START:  .dword _kernel_phys_start
KERNEL_PHYS_END:    .dword _kernel_phys_end

.global KERNEL_START
.global KERNEL_END
KERNEL_START:       .dword _kernel_start
KERNEL_END:         .dword _kernel_end

.global TEXT_START
.global TEXT_END
TEXT_START:         .dword _text_start
TEXT_END:           .dword _text_end

.global RODATA_START
.global RODATA_END
RODATA_START:       .dword _rodata_start
RODATA_END:         .dword _rodata_end
