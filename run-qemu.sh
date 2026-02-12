#!/usr/bin/env bash

xterm -fullscreen -e qemu-system-riscv32 \
  -machine virt \
  -nographic \
  -m 128M \
  -bios default \
  -kernel "$1"
