use crate::arch::{PAddr, VAddr};
use pastey::paste;

macro_rules! def_symbols {
    ($name:ident) => {
        paste! {
            #[doc = concat!(
                "Returns the virtual address of the `",
                stringify!([<_ $name _start>]),
                "` symbol at the current relocation.",
            )]
            #[inline]
            pub fn [<$name _start_v>]() -> VAddr {
                unsafe extern "C" {
                    static [<_ $name _start>]: u8;
                }
                VAddr::from_ptr(&raw const [<_ $name _start>])
            }

            #[doc = concat!(
                "Returns the physical address of the `",
                stringify!([<_ $name _start>]),
                "` symbol.",
            )]
            #[inline]
            pub fn [<$name _start_p>]() -> PAddr {
                unsafe extern "C" {
                    static [<_ $name _start>]: u8;
                }
                PAddr::new(
                    (&raw const ([<_ $name _start>]) as usize)
                        .wrapping_sub($crate::mem::vms::phys_to_kernel_dynslide())
                )
            }

            #[doc = concat!(
                "Returns the virtual address of the `",
                stringify!([<_ $name _end>]),
                "` symbol at the current relocation.",
            )]
            #[inline]
            pub fn [<$name _end_v>]() -> VAddr {
                unsafe extern "C" {
                    static [<_ $name _end>]: u8;
                }
                VAddr::from_ptr(&raw const [<_ $name _end>])
            }

            #[doc = concat!(
                "Returns the physical address of the `",
                stringify!([<_ $name _end>]),
                "` symbol.",
            )]
            #[inline]
            pub fn [<$name _end_p>]() -> PAddr {
                unsafe extern "C" {
                    static [<_ $name _end>]: u8;
                }
                PAddr::new(
                    (&raw const ([<_ $name _end>]) as usize)
                        .wrapping_sub($crate::mem::vms::phys_to_kernel_dynslide())
                )
            }

            #[doc = concat!(
                "Returns the size (in bytes) of the region between `",
                stringify!([<_ $name _start>]),
                "` and `",
                stringify!([<_ $name _end>]),
                "`.",
            )]
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
def_symbols!(bump_heap);
