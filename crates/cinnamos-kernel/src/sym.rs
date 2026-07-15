#[macro_export]
macro_rules! kernel_start_v {
    () => {{
        unsafe extern "C" {
            static _kernel_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_kernel_start)
    }};
}

#[macro_export]
macro_rules! kernel_start_p {
    () => {{
        unsafe extern "C" {
            static _kernel_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_kernel_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! kernel_end_v {
    () => {{
        unsafe extern "C" {
            static _kernel_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_kernel_end)
    }};
}

#[macro_export]
macro_rules! kernel_end_p {
    () => {{
        unsafe extern "C" {
            static _kernel_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_kernel_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! text_start_v {
    () => {{
        unsafe extern "C" {
            static _text_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_text_start)
    }};
}

#[macro_export]
macro_rules! text_start_p {
    () => {{
        unsafe extern "C" {
            static _text_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_text_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! text_end_v {
    () => {{
        unsafe extern "C" {
            static _text_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_text_end)
    }};
}

#[macro_export]
macro_rules! text_end_p {
    () => {{
        unsafe extern "C" {
            static _text_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_text_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! rodata_start_v {
    () => {{
        unsafe extern "C" {
            static _rodata_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_rodata_start)
    }};
}

#[macro_export]
macro_rules! rodata_start_p {
    () => {{
        unsafe extern "C" {
            static _rodata_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_rodata_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! rodata_end_v {
    () => {{
        unsafe extern "C" {
            static _rodata_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_rodata_end)
    }};
}

#[macro_export]
macro_rules! rodata_end_p {
    () => {{
        unsafe extern "C" {
            static _rodata_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_rodata_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! data_start_v {
    () => {{
        unsafe extern "C" {
            static _data_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_data_start)
    }};
}

#[macro_export]
macro_rules! data_start_p {
    () => {{
        unsafe extern "C" {
            static _data_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_data_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! data_end_v {
    () => {{
        unsafe extern "C" {
            static _data_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_data_end)
    }};
}

#[macro_export]
macro_rules! data_end_p {
    () => {{
        unsafe extern "C" {
            static _data_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_data_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! kmem_start_v {
    () => {{
        unsafe extern "C" {
            static _kmem_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_kmem_start)
    }};
}

#[macro_export]
macro_rules! kmem_start_p {
    () => {{
        unsafe extern "C" {
            static _kmem_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_kmem_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! kmem_end_v {
    () => {{
        unsafe extern "C" {
            static _kmem_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_kmem_end)
    }};
}

#[macro_export]
macro_rules! kmem_end_p {
    () => {{
        unsafe extern "C" {
            static _kmem_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_kmem_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! stack_start_v {
    () => {{
        unsafe extern "C" {
            static _stack_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_stack_start)
    }};
}

#[macro_export]
macro_rules! stack_start_p {
    () => {{
        unsafe extern "C" {
            static _stack_start: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_stack_start) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! stack_end_v {
    () => {{
        unsafe extern "C" {
            static _stack_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_stack_end)
    }};
}

#[macro_export]
macro_rules! stack_end_p {
    () => {{
        unsafe extern "C" {
            static _stack_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_stack_end) as usize).wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}

#[macro_export]
macro_rules! trap_stack_start_v {
    () => {{
        unsafe extern "C" {
            static _trap_stack_start: u8;
        }
        $crate::arch::VAddr::from_ptr(&_trap_stack_start)
    }};
}

#[macro_export]
macro_rules! trap_stack_start_p {
    () => ({
        unsafe extern "C" { static _trap_stack_start: u8; }
        $crate::arch::PAddr:new((&raw const _trap_stack_start() as usize).wrapping_sub($crate::phys_to_kernel_symshift!()))
    })
}

#[macro_export]
macro_rules! trap_stack_end_v {
    () => {{
        unsafe extern "C" {
            static _trap_stack_end: u8;
        }
        $crate::arch::VAddr::from_ptr(&_trap_stack_end)
    }};
}

#[macro_export]
macro_rules! trap_stack_end_p {
    () => {{
        unsafe extern "C" {
            static _trap_stack_end: u8;
        }
        $crate::arch::PAddr::new(
            (&raw const (_trap_stack_end) as usize)
                .wrapping_sub($crate::phys_to_kernel_symshift!()),
        )
    }};
}
