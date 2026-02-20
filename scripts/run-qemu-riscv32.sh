#!/usr/bin/env bash

xterm -fullscreen -e qemu-system-riscv32 \
  -machine virt \
  -nographic \
  -S -gdb tcp::1234 \
  -smp 1 \
  -m 128M \
  -bios default \
  -kernel "$1"
