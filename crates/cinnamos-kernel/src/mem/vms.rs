use core::mem::MaybeUninit;

use alloc::vec::Vec;
use fdt::Fdt;
use spin::RwLock;

use crate::{
    arch::*,
    mem::{
        PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion,
        addrsp::{AddressSpace, AddressSpaceError},
        physalloc::FrameAlloc,
        virt::VirtAlloc,
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
struct SendAddressSpace(AddressSpace<'static>);

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

/// Should only be called once in early phase
pub fn init(fdt: &Fdt, dtb_pa: PAddr) -> Result<VirtualMemoryInfo, VmsError> {
    if ROOT_ADDRSP.read().is_none() {
        let root_alloc = mem::physalloc::alloc(1).ok_or(VmsError::FrameAllocFailed)?;
        let root_ptr = VAddr::identity(root_alloc.start_addr()).as_mut::<MaybeUninit<_>>();
        let root_ptr = unsafe { PageTable::init(root_ptr.as_mut_unchecked()) };
        let root_addrsp = AddressSpace::new(
            0,
            root_ptr,
            root_alloc,
            Vec::with_capacity(32),
            &VAddr::identity,
        )
        .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} id-map text",
            VAddr::identity(text_start_p()),
            VAddr::identity(text_end_p()),
            text_start_p(),
            text_end_p(),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(text_start_p()),
                text_start_p(),
                text_size(),
                PTEFlags::RX,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} id-map rodata",
            VAddr::identity(rodata_start_p()),
            VAddr::identity(rodata_end_p()),
            rodata_start_p(),
            rodata_end_p(),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(rodata_start_p()),
                rodata_start_p(),
                rodata_size(),
                PTEFlags::READ,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} id-map data",
            VAddr::identity(data_start_p()),
            VAddr::identity(data_end_p()),
            data_start_p(),
            data_end_p(),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(data_start_p()),
                data_start_p(),
                data_size(),
                PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} id-map kmem",
            VAddr::identity(kmem_start_p()),
            VAddr::identity(kmem_end_p()),
            kmem_start_p(),
            kmem_end_p(),
        );
        root_addrsp
            .map_raw(
                VAddr::identity(kmem_start_p()),
                kmem_start_p(),
                kmem_size(),
                PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} id-map bump",
            VAddr::identity(bump_heap_start_p()),
            VAddr::identity(bump_heap_end_p()),
            bump_heap_start_p(),
            bump_heap_end_p(),
        );
        root_addrsp
            .map_raw_skip_mapped(
                VAddr::identity(bump_heap_start_p()),
                bump_heap_start_p(),
                bump_heap_size(),
                PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} hi-map text",
            phys_to_kernel(text_start_p()),
            phys_to_kernel(text_end_p()),
            text_start_p(),
            text_end_p(),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(text_start_p()),
                text_start_p(),
                text_size(),
                PTEFlags::GLOBAL | PTEFlags::RX,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} hi-map rodata",
            phys_to_kernel(rodata_start_p()),
            phys_to_kernel(rodata_end_p()),
            rodata_start_p(),
            rodata_end_p(),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(rodata_start_p()),
                rodata_start_p(),
                rodata_size(),
                PTEFlags::GLOBAL | PTEFlags::READ,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} hi-map data",
            phys_to_kernel(data_start_p()),
            phys_to_kernel(data_end_p()),
            data_start_p(),
            data_end_p(),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(data_start_p()),
                data_start_p(),
                data_size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} hi-map kmem",
            phys_to_kernel(kmem_start_p()),
            phys_to_kernel(kmem_end_p()),
            kmem_start_p(),
            kmem_end_p(),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(kmem_start_p()),
                kmem_start_p(),
                kmem_size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} hi-map bump",
            phys_to_kernel(bump_heap_start_p()),
            phys_to_kernel(bump_heap_end_p()),
            bump_heap_start_p(),
            bump_heap_end_p(),
        );
        root_addrsp
            .map_raw(
                phys_to_kernel(bump_heap_start_p()),
                bump_heap_start_p(),
                bump_heap_size(),
                PTEFlags::GLOBAL | PTEFlags::RW,
            )
            .map_err(VmsError::AddressSpace)?;

        let (mut usable_regs, _) = devicetree::get_region_slices(
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
        let (bump_start, _, bump_end) = mem::alloc::bump::get_bump_area();
        if let Some(bump_reg) = SizedMemoryRegion::from_range(
            bump_start.align_to_page(),
            bump_end.align_to_next_page(),
        ) {
            usable_regs.push(bump_reg);
        }

        for r in &usable_regs {
            let pa = r.base;
            let pa_end = r.end();

            log::debug!(
                "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} di-map mem",
                phys_to_virt(pa),
                phys_to_virt(pa_end),
                pa,
                pa_end,
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

        log::debug!(
            "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} di-map dtb",
            phys_to_virt(dtb_pa),
            phys_to_virt(dtb_pa + fdt.total_size()),
            dtb_pa,
            dtb_pa + fdt.total_size(),
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

                            log::debug!(
                                "{:#016x} .. {:#016x} -> {:#016x} .. {:#016x} di-map /soc/{}",
                                phys_to_virt(pa),
                                phys_to_virt(pa_end),
                                pa,
                                pa_end,
                                n.name,
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
            log::debug!("testing mappings");

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
            log::debug!("phys-to-kernel translation success");

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
            log::debug!("phys-to-direct translation success");

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
            log::debug!("identity-vtmap translation success");
        }

        let mut g = ROOT_ADDRSP.write();
        if g.is_none() {
            let max_asid = arch::get_max_asid();
            arch::switch_address_space(&root_addrsp);
            *g = Some(SendAddressSpace(root_addrsp));
            Ok(VirtualMemoryInfo { max_asid })
        } else {
            Err(VmsError::RootTableAlreadyInitialized)
        }
    } else {
        Err(VmsError::RootTableAlreadyInitialized)
    }
}

/// Should only be called once from the kernel address space.
pub fn remap_tables() -> Result<(), VmsError> {
    log::debug!("remapping page tables");
    let mut g = ROOT_ADDRSP.write();
    let root_addrsp = g.as_mut().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp
        .0
        .remap(&phys_to_virt)
        .map_err(VmsError::AddressSpace)
}

/// Should only be called once after [remapping the address space](remap_tables) and [initializing the heap](mem::heap::init_heap).
pub fn uninit_identity_map() -> Result<(), VmsError> {
    let g = ROOT_ADDRSP.read();
    let root_addrsp = g.as_ref().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp.0.realloc();

    log::debug!(
        "unmapping id-map kernel: {:#016x} .. {:#016x}",
        kernel_start_p(),
        kernel_end_p()
    );
    root_addrsp
        .0
        .unmap_raw(VAddr::identity(kernel_start_p()), kernel_size())
        .map_err(VmsError::AddressSpace)
}

pub fn map(virt: &impl VirtAlloc, phys: &FrameAlloc, flags: PTEFlags) -> Result<(), VmsError> {
    let g = ROOT_ADDRSP.read();
    let root_addrsp = g.as_ref().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp
        .0
        .map(virt, phys, flags)
        .map_err(VmsError::AddressSpace)
}

pub fn map_raw(va: VAddr, pa: PAddr, size_bytes: usize, flags: PTEFlags) -> Result<(), VmsError> {
    let g = ROOT_ADDRSP.read();
    let root_addrsp = g.as_ref().ok_or(VmsError::RootTableUninitialized)?;
    root_addrsp
        .0
        .map_raw(va, pa, size_bytes, flags)
        .map_err(VmsError::AddressSpace)
}
