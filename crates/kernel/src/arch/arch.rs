pub trait Arch {
    fn init();
}

#[inline(always)]
pub fn init() {
    crate::arch::ArchImpl::init();
}
