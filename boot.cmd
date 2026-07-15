fdt addr ${fdtcontroladdr}
fdt resize
fdt move ${fdtcontroladdr} ${fdt_addr_r}
fatload virtio 0:1 ${kernel_addr_r} boot/kernel.fit
bootm ${kernel_addr_r} - ${fdt_addr_r}