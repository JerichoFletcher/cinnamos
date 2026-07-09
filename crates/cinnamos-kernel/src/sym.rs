#[macro_export]
macro_rules! kernel_start {
    () => ({
        unsafe extern "C" { static _kernel_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_kernel_start)
    })
}

#[macro_export]
macro_rules! kernel_end {
    () => ({
        unsafe extern "C" { static _kernel_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_kernel_end)
    })
}

#[macro_export]
macro_rules! text_start {
    () => ({
        unsafe extern "C" { static _text_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_text_start)
    })
}

#[macro_export]
macro_rules! text_end {
    () => ({
        unsafe extern "C" { static _text_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_text_end)
    })
}

#[macro_export]
macro_rules! rodata_start {
    () => ({
        unsafe extern "C" { static _rodata_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_rodata_start)
    })
}

#[macro_export]
macro_rules! rodata_end {
    () => ({
        unsafe extern "C" { static _rodata_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_rodata_end)
    })
}

#[macro_export]
macro_rules! bss_start {
    () => ({
        unsafe extern "C" { static _bss_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_bss_start)
    })
}

#[macro_export]
macro_rules! bss_end {
    () => ({
        unsafe extern "C" { static _bss_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_bss_end)
    })
}

#[macro_export]
macro_rules! data_start {
    () => ({
        unsafe extern "C" { static _data_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_data_start)
    })
}

#[macro_export]
macro_rules! data_end {
    () => ({
        unsafe extern "C" { static _data_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_data_end)
    })
}

#[macro_export]
macro_rules! kmem_start {
    () => ({
        unsafe extern "C" { static _kmem_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_kmem_start)
    })
}

#[macro_export]
macro_rules! kmem_end {
    () => ({
        unsafe extern "C" { static _kmem_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_kmem_end)
    })
}

#[macro_export]
macro_rules! stack_start {
    () => ({
        unsafe extern "C" { static _stack_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_stack_start)
    })
}

#[macro_export]
macro_rules! stack_end {
    () => ({
        unsafe extern "C" { static _stack_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_stack_end)
    })
}

#[macro_export]
macro_rules! trap_stack_start {
    () => ({
        unsafe extern "C" { static _trap_stack_start: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_trap_stack_start)
    })
}

#[macro_export]
macro_rules! trap_stack_end {
    () => ({
        unsafe extern "C" { static _trap_stack_end: [u8; 1]; }
        $crate::arch::PAddr::from_ptr(&_trap_stack_end)
    })
}
