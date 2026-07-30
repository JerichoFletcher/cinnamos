use core::mem::MaybeUninit;

use alloc::vec::Vec;
use fdt::Fdt;
use spin::RwLock;

use crate::{
    arch::*,
    mem::{
        PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion,
        addrsp::{AddressSpace, AddressSpaceError},
        virt::buddy::BuddyPageAlloc,
    },
    sym::*,
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmsError {
    FrameAllocFailed,
    RootTableUninitialized,
    RootTableAlreadyInitialized,
    Unaligned,
    AddressSpace(AddressSpaceError),
}

#[derive(Debug)]
struct SendAddressSpace(AddressSpace<'static, BuddyPageAlloc>);

// impl SendAddressSpace {
//     fn root_pt_pa(&self) -> PAddr {
//         self.0.root_pa()
//     }

//     fn root_pt(&self) -> *mut PageTable {
//         self.0.root_ptr()
//     }
// }

unsafe impl Send for SendAddressSpace {}
unsafe impl Sync for SendAddressSpace {}

static ROOT_ADDRSP: RwLock<Option<SendAddressSpace>> = RwLock::new(None);

pub struct VirtualMemoryInfo {
    pub max_asid: usize,
}

pub fn phys_to_kernel_dynslide() -> usize {
    kernel_start_v().addr().wrapping_sub(KERNEL_LOAD_BASE)
}

pub const PHYS_TO_KERNEL_SLIDE: usize = KERNEL_MAP_BASE - KERNEL_LOAD_BASE;

pub fn phys_to_kernel(pa: PAddr) -> VAddr {
    VAddr::new(pa.addr().wrapping_add(PHYS_TO_KERNEL_SLIDE))
}

pub fn kernel_to_phys(va: VAddr) -> PAddr {
    PAddr::new(va.addr().wrapping_sub(PHYS_TO_KERNEL_SLIDE))
}

pub fn phys_to_virt(pa: PAddr) -> VAddr {
    VAddr::new(pa.addr().wrapping_add(DIRECT_MAP_BASE))
}

pub fn virt_to_phys(va: VAddr) -> PAddr {
    PAddr::new(va.addr().wrapping_sub(DIRECT_MAP_BASE))
}

// fn map_and_forget(
//     root_pt: *mut PageTable,
//     pa_start: PAddr,
//     pa_end: PAddr,
//     va: VAddr,
//     flags: PTEFlags,
//     p2v: &impl Fn(PAddr) -> VAddr,
// ) -> Result<(), VmsError> {
//     let mut pa = pa_start;
//     let mut va = va;
//     while pa < pa_end {
//         let next_size = PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
//         arch::map_page(root_pt, va, pa, next_size, flags, p2v)
//             .map_err(VmsError::Map)?
//             .forget();
//         pa = pa + next_size.size();
//         va = va + next_size.size();
//     }
//     Ok(())
// }

// fn map_and_take_allocs(
//     root_pt: *mut PageTable,
//     pa_start: PAddr,
//     pa_end: PAddr,
//     va: VAddr,
//     flags: PTEFlags,
//     p2v: &impl Fn(PAddr) -> VAddr,
//     alloc_out: &mut impl Extend<Alloc>,
// ) -> Result<(), VmsError> {
//     let mut pa = pa_start;
//     let mut va = va;
//     while pa < pa_end {
//         let next_size = PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
//         alloc_out.extend(
//             arch::map_page(root_pt, va, pa, next_size, flags, p2v)
//                 .map_err(VmsError::Map)?
//                 .take_new_allocs(),
//         );
//         pa = pa + next_size.size();
//         va = va + next_size.size();
//     }
//     Ok(())
// }

/// Should only be called once in early phase
pub fn init(fdt: &Fdt, dtb_pa: PAddr) -> Result<VirtualMemoryInfo, VmsError> {
    let mut g = ROOT_ADDRSP.write();
    if g.as_mut().is_none() {
        let root_alloc = mem::physalloc::alloc(1).ok_or(VmsError::FrameAllocFailed)?;
        let root_ptr = VAddr::identity(root_alloc.start_addr()).as_mut::<MaybeUninit<_>>();
        let root_ptr = unsafe { PageTable::init(root_ptr.as_mut_unchecked()) };
        let root_addrsp = AddressSpace::new(
            root_ptr,
            root_alloc,
            Vec::with_capacity(32),
            &VAddr::identity,
        )
        .map_err(VmsError::AddressSpace)?;

        println!(
            "id-map text\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            text_start_p(),
            text_end_p(),
            VAddr::identity(text_start_p()),
            VAddr::identity(text_end_p()),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(text_start_p()),
                text_start_p(),
                text_size(),
                PTEFlags::RX,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "id-map rodata\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            rodata_start_p(),
            rodata_end_p(),
            VAddr::identity(rodata_start_p()),
            VAddr::identity(rodata_end_p()),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(rodata_start_p()),
                rodata_start_p(),
                rodata_size(),
                PTEFlags::READ,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "id-map data\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            data_start_p(),
            data_end_p(),
            VAddr::identity(data_start_p()),
            VAddr::identity(data_end_p()),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(data_start_p()),
                data_start_p(),
                data_size(),
                PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "id-map kmem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            kmem_start_p(),
            kmem_end_p(),
            VAddr::identity(kmem_start_p()),
            VAddr::identity(kmem_end_p()),
        );
        root_addrsp
            .map_raw_skip_mapped(
                VAddr::identity(kmem_start_p()),
                kmem_start_p(),
                kmem_size(),
                PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "hi-map text\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            text_start_p(),
            text_end_p(),
            phys_to_kernel(text_start_p()),
            phys_to_kernel(text_end_p()),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(text_start_p()),
                text_start_p(),
                text_size(),
                PTEFlags::GLOBAL | PTEFlags::RX,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "hi-map rodata\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            rodata_start_p(),
            rodata_end_p(),
            phys_to_kernel(rodata_start_p()),
            phys_to_kernel(rodata_end_p()),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(rodata_start_p()),
                rodata_start_p(),
                rodata_size(),
                PTEFlags::GLOBAL | PTEFlags::READ,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "hi-map data\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            data_start_p(),
            data_end_p(),
            phys_to_kernel(data_start_p()),
            phys_to_kernel(data_end_p()),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(data_start_p()),
                data_start_p(),
                data_size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        println!(
            "hi-map kmem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            kmem_start_p(),
            kmem_end_p(),
            phys_to_kernel(kmem_start_p()),
            phys_to_kernel(kmem_end_p()),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(kmem_start_p()),
                kmem_start_p(),
                kmem_size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        let (usable_regs, _) = devicetree::get_region_slices(
            fdt,
            [
                // Safety: Used symbols are defined in the linker script
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        kernel_start_p(),
                        kernel_end_p() - kernel_start_p(),
                    )
                },
                // Safety: The size of the devicetree blob is nonzero
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        dtb_pa,
                        fdt.total_size().next_multiple_of(PAGE_SIZE),
                    )
                },
            ],
        );

        for r in &usable_regs {
            let pa = r.base;
            let pa_end = r.end();

            println!(
                "di-map mem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                pa,
                pa_end,
                phys_to_virt(pa),
                phys_to_virt(pa_end),
            );
            root_addrsp
                .map_raw(
                    phys_to_virt(pa),
                    pa,
                    r.size,
                    PTEFlags::GLOBAL | PTEFlags::RW,
                )
                .map_err(VmsError::AddressSpace)?;
        }

        println!(
            "di-map dtb\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            dtb_pa,
            dtb_pa + fdt.total_size(),
            phys_to_virt(dtb_pa),
            phys_to_virt(dtb_pa + fdt.total_size()),
        );
        root_addrsp
            .map_raw(
                phys_to_virt(dtb_pa),
                dtb_pa,
                fdt.total_size(),
                PTEFlags::GLOBAL | PTEFlags::READ,
            )
            .map_err(VmsError::AddressSpace)?;

        if let Some(soc) = fdt.find_node("/soc") {
            for n in soc.children() {
                if let Some(regs) = n.reg() {
                    for r in regs {
                        if let Some(size) = r.size {
                            let pa = PAddr::from_ptr(r.starting_address);
                            let pa_end = pa + size;

                            println!(
                                "di-map /soc/{}\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                                n.name,
                                pa,
                                pa_end,
                                phys_to_virt(pa),
                                phys_to_virt(pa_end),
                            );
                            root_addrsp
                                .map_raw(
                                    phys_to_virt(pa),
                                    pa,
                                    size,
                                    PTEFlags::GLOBAL | PTEFlags::RW,
                                )
                                .map_err(VmsError::AddressSpace)?;
                        }
                    }
                }
            }
        }

        #[cfg(debug_assertions)]
        {
            use crate::sym::{kernel_end_p, kernel_start_p};

            println!("debug : testing mappings");

            let mut pa_orig = kernel_start_p();
            while pa_orig < kernel_end_p() {
                let va = phys_to_kernel(pa_orig);
                let pa_trns = arch::translate_virt(root_ptr, va, VAddr::identity);
                let va_addr = va.addr();
                let pa_orig_addr = pa_orig.addr();
                let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                debug_assert_eq!(
                    pa_trns,
                    Some(pa_orig),
                    "Phys-to-kernel translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}"
                );

                pa_orig = pa_orig + PAGE_SIZE;
            }
            println!("debug : phys-to-kernel translation success");

            for r in usable_regs {
                pa_orig = r.base;
                while pa_orig < r.end() {
                    let va = phys_to_virt(pa_orig);
                    let pa_trns = arch::translate_virt(root_ptr, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(
                        pa_trns,
                        Some(pa_orig),
                        "Phys-to-direct translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}"
                    );

                    pa_orig = pa_orig + PAGE_SIZE;
                }
            }
            println!("debug : phys-to-direct translation success");

            pa_orig = kernel_start_p();
            while pa_orig < kernel_end_p() {
                let va = VAddr::identity(pa_orig);
                let pa_trns = arch::translate_virt(root_ptr, va, VAddr::identity);
                let va_addr = va.addr();
                let pa_orig_addr = pa_orig.addr();
                let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                debug_assert_eq!(
                    pa_trns,
                    Some(pa_orig),
                    "Identity-vtmap translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}"
                );

                pa_orig = pa_orig + PAGE_SIZE;
            }
            println!("debug : identity-vtmap translation success");
        }

        let max_asid = arch::activate_vmap(root_addrsp.root_pa());
        *g = Some(SendAddressSpace(root_addrsp));
        Ok(VirtualMemoryInfo { max_asid })
    } else {
        Err(VmsError::RootTableAlreadyInitialized)
    }
}

/// Should only be called once from the kernel address space.
pub fn remap_tables() -> Result<(), VmsError> {
    let mut g = ROOT_ADDRSP.write();
    let root_addrsp = g.as_mut().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp.0.remap(&phys_to_virt).map_err(VmsError::AddressSpace)
}

/// Should only be called once after [remapping the address space](remap_tables) and [initializing the heap](mem::heap::init_heap).
pub fn uninit_identity_map() -> Result<(), VmsError> {
    let g = ROOT_ADDRSP.read();
    let root_addrsp = g.as_ref().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp.0.realloc();

    println!(
        "unmapping id-map kernel\t: 0x{:016x} .. 0x{:016x}",
        kernel_start_p(),
        kernel_end_p()
    );
    root_addrsp
        .0
        .unmap_raw(VAddr::identity(kernel_start_p()), kernel_size())
        .map_err(VmsError::AddressSpace)
}

// pub struct VmsAccessGuard<'a, F>
// where
//     F: Fn(PAddr) -> VAddr,
// {
//     guard: MutexGuard<'a, Option<SendAddressSpace>, Spin>,
//     p2v: &'a F,
// }

// impl<F: Fn(PAddr) -> VAddr> VmsAccessGuard<'_, F> {
//     pub fn root_pt_pa(&self) -> Result<PAddr, VmsError> {
//         let wrapper = self
//             .guard
//             .as_ref()
//             .ok_or(VmsError::RootTableUninitialized)?;
//         Ok(wrapper.0.root_pa())
//     }

//     pub fn root_pt(&mut self) -> Result<*mut PageTable, VmsError> {
//         let wrapper = self
//             .guard
//             .as_mut()
//             .ok_or(VmsError::RootTableUninitialized)?;
//         Ok(wrapper.0.root_ptr())
//     }

//     pub fn map_page(
//         &mut self,
//         va: VAddr,
//         pa: PAddr,
//         size: PageSize,
//         flags: PTEFlags,
//     ) -> Result<PageTableAllocMap, VmsError> {
//         let p2v = self.p2v;
//         let root_pt = self.root_pt()?;

//         let allocs = arch::map_page(root_pt, va, pa, size, flags, p2v).map_err(VmsError::Map)?;
//         Ok(allocs)
//     }

//     pub fn unmap_page(&mut self, va: VAddr) -> Result<PageSize, VmsError> {
//         let p2v = self.p2v;
//         let root_pt = self.root_pt()?;

//         let unmapped_size = arch::unmap_page(root_pt, va, p2v).map_err(VmsError::Unmap)?;
//         Ok(unmapped_size)
//     }

//     pub fn map_pages_and_forget(
//         &mut self,
//         pa_start: PAddr,
//         pa_end: PAddr,
//         va: VAddr,
//         flags: PTEFlags,
//     ) -> Result<(), VmsError> {
//         let p2v = self.p2v;
//         let root_pt = self.root_pt()?;

//         map_and_forget(root_pt, pa_start, pa_end, va, flags, p2v)
//     }

//     pub fn map_pages_and_take_alloc(
//         &mut self,
//         pa_start: PAddr,
//         pa_end: PAddr,
//         va: VAddr,
//         flags: PTEFlags,
//         alloc_out: &mut impl Extend<Alloc>,
//     ) -> Result<(), VmsError> {
//         let p2v = self.p2v;
//         let root_pt = self.root_pt()?;

//         map_and_take_allocs(root_pt, pa_start, pa_end, va, flags, p2v, alloc_out)
//     }

//     pub fn unmap_pages(&mut self, va: VAddr, size_bytes: usize) -> Result<(), VmsError> {
//         let mut va = va;
//         let va_end = va + size_bytes;
//         while va < va_end {
//             let next_size = self.unmap_page(va)?;
//             va = va + next_size.size();
//         }
//         Ok(())
//     }
// }

pub fn map_raw(va: VAddr, pa: PAddr, size_bytes: usize, flags: PTEFlags) -> Result<(), VmsError> {
    let g = ROOT_ADDRSP.read();
    let root_addrsp = g.as_ref().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp
        .0
        .map_raw(va, pa, size_bytes, flags)
        .map_err(VmsError::AddressSpace)
}

// pub fn acquire_with_p2v<F, T>(p2v: F, f: impl FnOnce(VmsAccessGuard<'_, F>) -> T) -> T
// where
//     F: Fn(PAddr) -> VAddr,
// {
//     let guard = ROOT_ADDRSP.lock();
//     let guard = VmsAccessGuard { guard, p2v: &p2v };
//     f(guard)
// }

// pub fn acquire<T>(f: impl FnOnce(VmsAccessGuard<'_, fn(PAddr) -> VAddr>) -> T) -> T {
//     let p2v: fn(PAddr) -> VAddr = phys_to_virt;
//     acquire_with_p2v(p2v, f)
// }
