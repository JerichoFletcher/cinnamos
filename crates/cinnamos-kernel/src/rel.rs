use crate::mem::vms::PHYS_TO_KERNEL_SLIDE;

#[repr(C)]
pub struct Elf64Dyn {
    pub tag: u64,
    pub val: u64,
}

pub const DT_NULL: u64 = 0;
pub const DT_RELA: u64 = 7;
pub const DT_RELASZ: u64 = 8;
pub const DT_RELAENT: u64 = 9;

#[repr(C)]
pub struct Elf64Rela {
    pub offset: u64,
    pub info: u64,
    pub addend: i64,
}

pub const R_RISCV64_RELA: u64 = 3;

#[inline(always)]
fn relocate_entry(rela: &Elf64Rela) {
    let kernel_to_phys_slide = PHYS_TO_KERNEL_SLIDE.wrapping_neg();
    let rela_type = rela.info & 0xffffffff;

    match rela_type {
        R_RISCV64_RELA => {
            let target = (rela.offset as usize).wrapping_add(kernel_to_phys_slide) as *mut usize;
            unsafe {
                *target = (rela.addend as usize).wrapping_add(kernel_to_phys_slide) as usize;
            }
        }
        _ => (),
    }
}

#[inline(always)]
unsafe fn slide_entry(rela: &Elf64Rela, slide: usize) {
    let rela_type = rela.info & 0xffffffff;
    match rela_type {
        R_RISCV64_RELA => {
            let target = rela.offset as *mut usize;
            unsafe {
                let paddr = *target;
                *target = paddr + slide;
            }
        }
        _ => (),
    }
}

/// # Safety
/// `dyn_ptr` must point to the `_DYNAMIC` symbol.
#[inline(always)]
pub unsafe fn relocate(dyn_ptr: *const Elf64Dyn) {
    let mut rela_addr: *const Elf64Rela = core::ptr::null();
    let mut rela_size = 0usize;
    let mut rela_ent_size = 0usize;

    let kernel_to_phys_slide = PHYS_TO_KERNEL_SLIDE.wrapping_neg();

    unsafe {
        let mut dyn_ptr = dyn_ptr;
        while (*dyn_ptr).tag != DT_NULL {
            match (*dyn_ptr).tag {
                DT_RELA => {
                    rela_addr = ((*dyn_ptr).val as usize).wrapping_add(kernel_to_phys_slide)
                        as *const Elf64Rela
                }
                DT_RELASZ => rela_size = (*dyn_ptr).val as usize,
                DT_RELAENT => rela_ent_size = (*dyn_ptr).val as usize,
                _ => (),
            }
            dyn_ptr = dyn_ptr.add(1);
        }

        if !rela_addr.is_null() && rela_size != 0 {
            let rela_ent_count = rela_size / rela_ent_size;
            for i in 0..rela_ent_count {
                let rela = &*((rela_addr as usize + i * rela_ent_size) as *const Elf64Rela);
                relocate_entry(rela);
            }
        }
    }
}

/// # Safety
/// `dyn_ptr` must point to the `_DYNAMIC` symbol.
#[inline(always)]
pub unsafe fn shift_relocation(dyn_ptr: *const Elf64Dyn, slide: usize) {
    let mut rela_addr: *const Elf64Rela = core::ptr::null();
    let mut rela_size = 0usize;
    let mut rela_ent_size = 0usize;

    unsafe {
        let mut dyn_ptr = dyn_ptr;
        while (*dyn_ptr).tag != DT_NULL {
            match (*dyn_ptr).tag {
                DT_RELA => rela_addr = (*dyn_ptr).val as *const Elf64Rela,
                DT_RELASZ => rela_size = (*dyn_ptr).val as usize,
                DT_RELAENT => rela_ent_size = (*dyn_ptr).val as usize,
                _ => (),
            }
            dyn_ptr = dyn_ptr.add(1);
        }

        if !rela_addr.is_null() && rela_size != 0 {
            let rela_ent_count = rela_size / rela_ent_size;
            for i in 0..rela_ent_count {
                let rela = &*((rela_addr as usize + i * rela_ent_size) as *const Elf64Rela);
                slide_entry(rela, slide);
            }
        }
    }
}
