use core::mem::{ManuallyDrop, MaybeUninit};

use alloc::vec::Vec;
use fdt::Fdt;
use spin::{Mutex, MutexGuard, Spin};

use crate::{
    arch::*,
    mem::{PAGE_SIZE, PhysFrameAlloc, SizedMemoryRegion, palloc::Alloc},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmsError {
    FrameAllocFailed,
    RootTableUninitialized,
    RootTableAlreadyInitialized,
    Unaligned,
    Map(MapError),
    Unmap(UnmapError),
}

#[derive(Debug)]
enum SendRootTable {
    Raw(ManuallyDrop<Alloc>),
    Virtual(ManuallyDrop<Alloc>, *mut PageTable),
}

impl SendRootTable {
    fn root_pt_pa(&self) -> PAddr {
        match self {
            Self::Raw(alloc) => alloc.base_addr(),
            Self::Virtual(alloc, _) => alloc.base_addr(),
        }
    }

    fn root_pt(&mut self) -> *mut PageTable {
        match self {
            Self::Raw(alloc) => VAddr::identity(alloc.base_addr()).as_mut(),
            Self::Virtual(_, p) => *p,
        }
    }
}

unsafe impl Send for SendRootTable {}

static ROOT_PT: Mutex<Option<SendRootTable>> = Mutex::new(None);

pub struct VirtualMemoryInfo {
    pub max_asid: usize,
}

#[macro_export]
macro_rules! phys_to_kernel_dynslide {
    () => {{
        use $crate::{arch::KERNEL_LOAD_BASE, kernel_start_v};
        kernel_start_v!().addr().wrapping_sub(KERNEL_LOAD_BASE)
    }};
}

pub const PHYS_TO_KERNEL_SLIDE: usize = KERNEL_MAP_BASE - KERNEL_LOAD_BASE;

pub fn phys_identity(pa: PAddr) -> VAddr {
    VAddr::identity(pa)
}

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
pub fn init() -> Result<(), VmsError> {
    let mut g = ROOT_PT.lock();
    if let None = g.as_mut() {
        let b = mem::palloc::alloc(1).ok_or(VmsError::FrameAllocFailed)?;
        let pt = phys_identity(b.base_addr()).as_mut() as *mut MaybeUninit<PageTable>;
        unsafe {
            PageTable::init(pt.as_mut_unchecked());
        }
        *g = Some(SendRootTable::Raw(ManuallyDrop::new(b)));
    }
    Ok(())
}

pub fn init_kernel_map(fdt: &Fdt, dtb_pa: PAddr) -> Result<VirtualMemoryInfo, VmsError> {
    acquire_with_p2v(&VAddr::identity, |mut g| {
        let mut pt_frames = Vec::with_capacity(32);

        unsafe {
            println!(
                "id-map text\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                text_start_p!(),
                text_end_p!(),
                VAddr::identity(text_start_p!()),
                VAddr::identity(text_end_p!()),
            );
            let mut pa = text_start_p!();
            while pa < text_end_p!() {
                let va = VAddr::identity(pa);
                let next_size =
                    PageSize::select_size(va, pa, text_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::RX)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "id-map rodata\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                rodata_start_p!(),
                rodata_end_p!(),
                VAddr::identity(rodata_start_p!()),
                VAddr::identity(rodata_end_p!()),
            );
            let mut pa = rodata_start_p!();
            while pa < rodata_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, rodata_end_p!() - pa)
                    .ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::READ)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "id-map data\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                data_start_p!(),
                data_end_p!(),
                VAddr::identity(data_start_p!()),
                VAddr::identity(data_end_p!()),
            );
            let mut pa = data_start_p!();
            while pa < data_end_p!() {
                let va = VAddr::identity(pa);
                let next_size =
                    PageSize::select_size(va, pa, data_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "id-map kmem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                kmem_start_p!(),
                kmem_end_p!(),
                VAddr::identity(kmem_start_p!()),
                VAddr::identity(kmem_end_p!()),
            );
            let mut pa = kmem_start_p!();
            while pa < kmem_end_p!() {
                let va = VAddr::identity(pa);
                let next_size =
                    PageSize::select_size(va, pa, kmem_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "hi-map text\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                text_start_p!(),
                text_end_p!(),
                phys_to_kernel(text_start_p!()),
                phys_to_kernel(text_end_p!()),
            );
            let mut pa = text_start_p!();
            while pa < text_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size =
                    PageSize::select_size(va, pa, text_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::GLOBAL | PTEFlags::RX)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "hi-map rodata\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                rodata_start_p!(),
                rodata_end_p!(),
                phys_to_kernel(rodata_start_p!()),
                phys_to_kernel(rodata_end_p!()),
            );
            let mut pa = rodata_start_p!();
            while pa < rodata_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, rodata_end_p!() - pa)
                    .ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::GLOBAL | PTEFlags::READ)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "hi-map data\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                data_start_p!(),
                data_end_p!(),
                phys_to_kernel(data_start_p!()),
                phys_to_kernel(data_end_p!()),
            );
            let mut pa = data_start_p!();
            while pa < data_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size =
                    PageSize::select_size(va, pa, data_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::GLOBAL | PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }

            println!(
                "hi-map kmem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                kmem_start_p!(),
                kmem_end_p!(),
                phys_to_kernel(kmem_start_p!()),
                phys_to_kernel(kmem_end_p!()),
            );
            let mut pa = kmem_start_p!();
            while pa < kmem_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size =
                    PageSize::select_size(va, pa, kmem_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::GLOBAL | PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }
        }

        let (usable_regs, _) = devicetree::get_region_slices(
            fdt,
            [
                // Safety: Used symbols are defined in the linker script
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        kernel_start_p!(),
                        kernel_end_p!() - kernel_start_p!(),
                    )
                },
                // Safety: The size of the devicetree blob is nonzero
                unsafe {
                    SizedMemoryRegion::new_unchecked(
                        dtb_pa,
                        (fdt.total_size() + PAGE_SIZE - 1) & !(PAGE_SIZE - 1),
                    )
                },
            ],
        );

        for r in &usable_regs {
            let mut pa = r.base;
            let pa_end = r.end();

            println!(
                "di-map mem\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                pa,
                pa_end,
                phys_to_virt(pa),
                phys_to_virt(pa_end),
            );
            while pa < pa_end {
                let va = phys_to_virt(pa);
                let next_size =
                    PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }
        }

        println!(
            "di-map dtb\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            dtb_pa,
            dtb_pa + fdt.total_size(),
            phys_to_virt(dtb_pa),
            phys_to_virt(dtb_pa + fdt.total_size()),
        );
        let mut pa = dtb_pa;
        let pa_end = dtb_pa + fdt.total_size();
        while pa < pa_end {
            let va = phys_to_virt(pa);
            let next_size =
                PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
            pt_frames.extend(
                g.map_page(va, pa, next_size, PTEFlags::READ)?
                    .take_new_allocs(),
            );
            pa = pa + next_size.size();
        }

        if let Some(soc) = fdt.find_node("/soc") {
            for n in soc.children() {
                if let Some(regs) = n.reg() {
                    for r in regs {
                        if let Some(size) = r.size {
                            let mut pa = PAddr::from_ptr(r.starting_address);
                            let pa_end = pa + size;

                            println!(
                                "di-map /soc/{}\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                                n.name,
                                pa,
                                pa_end,
                                phys_to_virt(pa),
                                phys_to_virt(pa_end),
                            );
                            while pa < pa_end {
                                let va = phys_to_virt(pa);
                                let next_size = PageSize::select_size(va, pa, pa_end - pa)
                                    .ok_or(VmsError::Unaligned)?;
                                pt_frames.extend(
                                    g.map_page(va, pa, next_size, PTEFlags::RW)?
                                        .take_new_allocs(),
                                );
                                pa = pa + next_size.size();
                            }
                        }
                    }
                }
            }
        }

        let mut pa = g.root_pt_pa()?;
        let pa_end = pa + PAGE_SIZE;
        println!(
            "di-map pt root\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
            pa,
            pa_end,
            phys_to_virt(pa),
            phys_to_virt(pa_end),
        );
        while pa < pa_end {
            let va = phys_to_virt(pa);
            let next_size =
                PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
            pt_frames.extend(
                g.map_page(va, pa, next_size, PTEFlags::RW)?
                    .take_new_allocs(),
            );
            pa = pa + next_size.size();
        }

        let mut i = 0usize;
        while !pt_frames.is_empty() {
            let alloc = pt_frames.pop().unwrap();
            let mut pa = alloc.base_addr();
            let pa_end = pa + size_of::<PageTable>();
            println!(
                "di-map pt {}\t: 0x{:016x} .. 0x{:016x} <- 0x{:016x} .. 0x{:016x}",
                i,
                pa,
                pa_end,
                phys_to_virt(pa),
                phys_to_virt(pa_end),
            );
            while pa < pa_end {
                let va = phys_to_virt(pa);
                let next_size =
                    PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
                pt_frames.extend(
                    g.map_page(va, pa, next_size, PTEFlags::RW)?
                        .take_new_allocs(),
                );
                pa = pa + next_size.size();
            }
            // TODO: Store in kernel own AddressSpace struct
            core::mem::forget(alloc);
            i += 1;
        }

        #[cfg(debug_assertions)]
        {
            use crate::{kernel_end_p, kernel_start_p};

            println!("debug : testing mappings");

            let mut pa_orig = unsafe { kernel_start_p!() };
            while pa_orig < unsafe { kernel_end_p!() } {
                let va = phys_to_kernel(pa_orig);
                let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
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
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
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

            pa_orig = unsafe { kernel_start_p!() };
            while pa_orig < unsafe { kernel_end_p!() } {
                let va = VAddr::identity(pa_orig);
                let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
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

        let root_pt_pa = g.root_pt_pa()?;
        g.attach_virt(phys_to_virt(root_pt_pa))?;

        let max_asid = arch::activate_vmap(root_pt_pa);
        Ok(VirtualMemoryInfo { max_asid })
    })
}

/// Should only be called from the kernel address space.
pub fn uninit_identity_map() -> Result<(), VmsError> {
    acquire(|mut g| {
        unsafe {
            println!(
                "unmapping id-map kernel\t: 0x{:016x} .. 0x{:016x}",
                kernel_start_p!(),
                kernel_end_p!()
            );
            let mut pa = kernel_start_p!();
            while pa < kernel_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = g.unmap_page(va)?;
                pa = pa + next_size.size();
            }
        }
        Ok(())
    })
}

pub struct VmsAccessGuard<'a, F>
where
    F: Fn(PAddr) -> VAddr,
{
    guard: MutexGuard<'a, Option<SendRootTable>, Spin>,
    p2v: &'a F,
    flush: bool,
}

impl<F: Fn(PAddr) -> VAddr> Drop for VmsAccessGuard<'_, F> {
    fn drop(&mut self) {
        if self.flush {
            arch::flush_vmap();
        }
    }
}

impl<F: Fn(PAddr) -> VAddr> VmsAccessGuard<'_, F> {
    pub fn root_pt_pa(&self) -> Result<PAddr, VmsError> {
        let wrapper = self
            .guard
            .as_ref()
            .ok_or(VmsError::RootTableUninitialized)?;
        Ok(wrapper.root_pt_pa())
    }

    pub fn root_pt(&mut self) -> Result<&mut PageTable, VmsError> {
        let wrapper = self
            .guard
            .as_mut()
            .ok_or(VmsError::RootTableUninitialized)?;
        // Safety: The root page table pointer is never null and always valid; the mutable reference is locked behind a mutex
        unsafe { Ok(wrapper.root_pt().as_mut_unchecked()) }
    }

    pub fn map_page(
        &mut self,
        va: VAddr,
        pa: PAddr,
        size: PageSize,
        flags: PTEFlags,
    ) -> Result<PageTableAllocMap, VmsError> {
        let p2v = self.p2v;
        let root_pt = self.root_pt()?;

        let allocs =
            arch::map_page(root_pt, va, pa, size, flags, p2v).map_err(|e| VmsError::Map(e))?;
        self.flush = true;
        Ok(allocs)
    }

    pub fn unmap_page(&mut self, va: VAddr) -> Result<PageSize, VmsError> {
        let p2v = self.p2v;
        let root_pt = self.root_pt()?;

        let unmapped_size = arch::unmap_page(root_pt, va, p2v).map_err(|e| VmsError::Unmap(e))?;
        self.flush = true;
        Ok(unmapped_size)
    }

    fn attach_virt(&mut self, va: VAddr) -> Result<(), VmsError> {
        if self.guard.is_none() {
            return Err(VmsError::RootTableUninitialized);
        }

        let old = self.guard.take().unwrap();
        *self.guard = Some(match old {
            SendRootTable::Raw(alloc) => SendRootTable::Virtual(alloc, va.as_mut()),
            SendRootTable::Virtual(_, _) => return Err(VmsError::RootTableAlreadyInitialized),
        });
        Ok(())
    }
}

pub fn acquire_with_p2v<F, T>(p2v: F, f: impl FnOnce(VmsAccessGuard<'_, F>) -> T) -> T
where
    F: Fn(PAddr) -> VAddr,
{
    let guard = ROOT_PT.lock();
    let guard = VmsAccessGuard {
        guard,
        p2v: &p2v,
        flush: false,
    };
    f(guard)
}

pub fn acquire<T>(f: impl FnOnce(VmsAccessGuard<'_, fn(PAddr) -> VAddr>) -> T) -> T {
    let p2v: fn(PAddr) -> VAddr = phys_to_virt;
    acquire_with_p2v(p2v, f)
}
