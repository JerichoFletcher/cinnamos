use elf::{
    abi::{DT_NULL, DT_RELA, DT_RELAENT, DT_RELASZ, R_RISCV_RELATIVE},
    relocation::Elf64_Rela,
};

use crate::{arch, mem::vms::PHYS_TO_KERNEL_SLIDE};

#[inline(always)]
fn relocate_entry(rela: &Elf64_Rela) {
    let rela_type = (rela.r_info & 0xffffffff) as u32;
    if rela_type == R_RISCV_RELATIVE {
        let target = (rela.r_offset as usize).wrapping_sub(PHYS_TO_KERNEL_SLIDE) as *mut usize;
        unsafe { *target = (rela.r_addend as usize).wrapping_sub(PHYS_TO_KERNEL_SLIDE) };
    }
}

#[inline(always)]
unsafe fn slide_entry(rela: &Elf64_Rela, slide: usize) {
    let rela_type = (rela.r_info & 0xffffffff) as u32;
    if rela_type == R_RISCV_RELATIVE {
        let target = rela.r_offset as *mut usize;
        unsafe { *target += slide };
    }
}

/// Performs dynamic relocation by filling in the global offset table entries.
///
/// This function will relocate kernel symbols to their load addresses.
#[inline(always)]
pub fn relocate() {
    let dyn_ptr = arch::get_dyn();
    let mut rela_addr: *const Elf64_Rela = core::ptr::null();
    let mut rela_size = 0usize;
    let mut rela_ent_size = 0usize;

    // Safety: All operations are safe given that dyn_ptr initially points to _DYNAMIC
    unsafe {
        let mut dyn_ptr = dyn_ptr;
        while (*dyn_ptr).d_tag != DT_NULL {
            match (*dyn_ptr).d_tag {
                DT_RELA => {
                    rela_addr = ((*dyn_ptr).d_un as usize).wrapping_sub(PHYS_TO_KERNEL_SLIDE)
                        as *const Elf64_Rela
                }
                DT_RELASZ => rela_size = (*dyn_ptr).d_un as usize,
                DT_RELAENT => rela_ent_size = (*dyn_ptr).d_un as usize,
                _ => (),
            }
            dyn_ptr = dyn_ptr.add(1);
        }

        if !rela_addr.is_null() && rela_size != 0 {
            let rela_ent_count = rela_size / rela_ent_size;
            for i in 0..rela_ent_count {
                let rela = &*((rela_addr as usize + i * rela_ent_size) as *const Elf64_Rela);
                relocate_entry(rela);
            }
        }
    }
}

/// Shifts all relocated symbol offsets by the given slide amount.
///
/// # Safety
/// `slide` must be equal to the difference between a kernel virtual address
/// and the physical address it is mapped to (the kernel space's slide amount).
#[inline(always)]
pub unsafe fn shift_relocation(slide: usize) {
    let dyn_ptr = arch::get_dyn();
    let mut rela_addr: *const Elf64_Rela = core::ptr::null();
    let mut rela_size = 0usize;
    let mut rela_ent_size = 0usize;

    // Safety: All operations are safe given that dyn_ptr initially points to _DYNAMIC
    unsafe {
        let mut dyn_ptr = dyn_ptr;
        while (*dyn_ptr).d_tag != DT_NULL {
            match (*dyn_ptr).d_tag {
                DT_RELA => rela_addr = (*dyn_ptr).d_un as *const Elf64_Rela,
                DT_RELASZ => rela_size = (*dyn_ptr).d_un as usize,
                DT_RELAENT => rela_ent_size = (*dyn_ptr).d_un as usize,
                _ => (),
            }
            dyn_ptr = dyn_ptr.add(1);
        }

        if !rela_addr.is_null() && rela_size != 0 {
            let rela_ent_count = rela_size / rela_ent_size;
            for i in 0..rela_ent_count {
                let rela = &*((rela_addr as usize + i * rela_ent_size) as *const Elf64_Rela);
                slide_entry(rela, slide);
            }
        }
    }
}
