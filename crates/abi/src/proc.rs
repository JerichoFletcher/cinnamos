use crate::impl_alias;

impl_alias! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    ProcessId = usize
}

impl_alias! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    ThreadId = usize
}
