use crate::arch::{PAddr, VAddr};
use pastey::paste;

macro_rules! def_symbols {
    ($name:ident) => {
        paste! {
            #[inline]
            pub fn [<$name _start_v>]() -> VAddr {
                unsafe extern "C" {
                    static [<_ $name _start>]: u8;
                }
                VAddr::from_ptr(&raw const [<_ $name _start>])
            }

            #[inline]
            pub fn [<$name _start_p>]() -> PAddr {
                unsafe extern "C" {
                    static [<_ $name _start>]: u8;
                }
                PAddr::new((&raw const ([<_ $name _start>]) as usize).wrapping_sub($crate::phys_to_kernel_dynslide!()))
            }

            #[inline]
            pub fn [<$name _end_v>]() -> VAddr {
                unsafe extern "C" {
                    static [<_ $name _end>]: u8;
                }
                VAddr::from_ptr(&raw const [<_ $name _end>])
            }

            #[inline]
            pub fn [<$name _end_p>]() -> PAddr {
                unsafe extern "C" {
                    static [<_ $name _end>]: u8;
                }
                PAddr::new((&raw const ([<_ $name _end>]) as usize).wrapping_sub($crate::phys_to_kernel_dynslide!()))
            }

            #[inline]
            pub fn [<$name _size>]() -> usize {
                unsafe extern "C" {
                    static [<_ $name _start>]: u8;
                    static [<_ $name _end>]: u8;
                }
                (&raw const [<_ $name _end>] as usize) - (&raw const [<_ $name _start>] as usize)
            }
        }
    };
}

def_symbols!(kernel);
def_symbols!(text);
def_symbols!(rodata);
def_symbols!(data);
def_symbols!(bss);
def_symbols!(kmem);
def_symbols!(stack);
def_symbols!(trap_stack);
def_symbols!(bump_heap);
