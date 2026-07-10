use core::mem::ManuallyDrop;

use fdt::Fdt;
use spin::{Mutex, MutexGuard, Spin};

use crate::{arch::*, mem::{FrameAlloc, PAGE_SIZE, palloc::{self, Alloc}}, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmsError {
    FrameAllocFailed,
    RootTableUninitialized,
    Unaligned,
    Map(MapError),
    Unmap(UnmapError),
}

struct SendRootTable(ManuallyDrop<Alloc>);

impl SendRootTable {
    fn root_pt_pa(&self) -> PAddr {
        self.0.base_addr()
    }

    /// # Safety
    /// `p2v` must be a physical-to-virtual address translator that is valid for the currently active virtual map.
    unsafe fn root_pt(&mut self, p2v: impl Fn(PAddr) -> VAddr) -> &mut PageTable {
        unsafe { p2v(self.0.base_addr()).as_mut::<PageTable>().as_mut_unchecked() }
    }
}

unsafe impl Send for SendRootTable {}

pub struct VirtualMemoryInfo {
    pub max_asid: usize,
}

static ROOT_PT: Mutex<Option<SendRootTable>> = Mutex::new(None);

#[macro_export]
macro_rules! phys_to_kernel_symshift {
    () => ({
        use $crate::arch::KERNEL_LOAD_BASE;
        kernel_start_v!().addr().wrapping_sub(KERNEL_LOAD_BASE)
    })
}

pub const PHYS_TO_KERNEL_SLIDE: usize = KERNEL_MAP_BASE - KERNEL_LOAD_BASE;

pub fn phys_to_kernel(pa: PAddr) -> VAddr {
    VAddr::new(pa.addr() + PHYS_TO_KERNEL_SLIDE)
}

pub fn phys_to_virt(pa: PAddr) -> VAddr {
    VAddr::new(pa.addr() + DIRECT_MAP_BASE)
}

pub fn init() -> Result<(), VmsError> {
    if let Some(alloc) = palloc::alloc(size_of::<PageTable>()) {
        let pt = unsafe { PageTable::init(VAddr::identity(alloc.base_addr())) };
        *ROOT_PT.lock() = Some(SendRootTable(ManuallyDrop::new(alloc)));
        println!("ptable : 0x{:016x}", pt.addr());
        Ok(())
    } else {
        Err(VmsError::FrameAllocFailed)
    }
}

pub fn init_kernel_map(fdt: &Fdt) -> Result<VirtualMemoryInfo, VmsError> {
    acquire_with_p2v(&VAddr::identity, |mut g| {
        unsafe {
            println!("id-map text\t: 0x{:016x} .. 0x{:016x}", text_start_p!(), text_end_p!());
            let mut pa = text_start_p!();
            while pa < text_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, text_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RX)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-map rodata\t: 0x{:016x} .. 0x{:016x}", rodata_start_p!(), rodata_end_p!());
            let mut pa = rodata_start_p!();
            while pa < rodata_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, rodata_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::READ)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-map data\t: 0x{:016x} .. 0x{:016x}", data_start_p!(), data_end_p!());
            pa = data_start_p!();
            while pa < data_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, data_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-map kmem\t: 0x{:016x} .. 0x{:016x}", kmem_start_p!(), kmem_end_p!());
            pa = kmem_start_p!();
            while pa < kmem_end_p!() {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, kmem_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }

            println!("hi-map text\t: 0x{:016x} .. 0x{:016x}", text_start_p!(), text_end_p!());
            pa = text_start_p!();
            while pa < text_end_p!() {                
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, text_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RX)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-map rodata\t: 0x{:016x} .. 0x{:016x}", rodata_start_p!(), rodata_end_p!());
            pa = rodata_start_p!();
            while pa < rodata_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, rodata_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::READ)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-map data\t: 0x{:016x} .. 0x{:016x}", data_start_p!(), data_end_p!());
            pa = data_start_p!();
            while pa < data_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, data_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-map kmem\t: 0x{:016x} .. 0x{:016x}", kmem_start_p!(), kmem_end_p!());
            pa = kmem_start_p!();
            while pa < kmem_end_p!() {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, kmem_end_p!() - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }

            for r in fdt.memory().regions() {
                if let Some(size) = r.size {
                    pa = PAddr::from_ptr(r.starting_address);
                    let pa_end = pa + size;

                    println!("di-map mem\t: 0x{:016x} .. 0x{:016x}", pa, pa_end);
                    while pa < pa_end {
                        let va = phys_to_virt(pa);
                        let next_size = PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
                        g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                        pa = pa + next_size.size();
                    }
                }
            }

            if let Some(soc) = fdt.find_node("/soc") {
                for n in soc.children() {
                    if let Some(regs) = n.reg() {
                        for r in regs {
                            if let Some(size) = r.size {
                                pa = PAddr::from_ptr(r.starting_address);
                                let pa_end = pa + size;

                                println!("di-map /soc/{}\t: 0x{:016x} .. 0x{:016x}", n.name, pa, pa_end);
                                while pa < pa_end {
                                    let va = phys_to_virt(pa);
                                    let next_size = PageSize::select_size(va, pa, pa_end - pa).ok_or(VmsError::Unaligned)?;
                                    g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                                    pa = pa + next_size.size();
                                }
                            }
                        }
                    }
                }
            }

            #[cfg(debug_assertions)] {
                use crate::{kernel_start_p, kernel_end_p};

                println!("debug : testing mappings");

                let mut pa_orig = kernel_start_p!();
                while pa_orig < kernel_end_p!() {
                    let va = phys_to_kernel(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Phys-to-kernel translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
                    
                    pa_orig = pa_orig + PAGE_SIZE;
                }
                println!("debug : phys-to-kernel translation success");
                
                pa_orig = kernel_start_p!();
                while pa_orig < kernel_end_p!() {
                    let va = phys_to_virt(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Phys-to-direct translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
                    
                    pa_orig = pa_orig + PAGE_SIZE;
                }
                println!("debug : phys-to-direct translation success");
                
                pa_orig = kernel_start_p!();
                while pa_orig < kernel_end_p!() {
                    let va = VAddr::identity(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Identity-vtmap translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
        
                    pa_orig = pa_orig + PAGE_SIZE;
                }
                println!("debug : identity-vtmap translation success");
            }

            let root_pt_pa = g.root_pt_pa()?;
            let max_asid = arch::activate_vmap(root_pt_pa);
            Ok(VirtualMemoryInfo { max_asid })
        }
    })
}

/// # Safety
/// This function can only be safely called from the kernel address space.
pub unsafe fn uninit_identity_map() -> Result<(), VmsError> {
    acquire(|mut g| {
        unsafe {
            println!("unmapping kernel\t: 0x{:016x} .. 0x{:016x}", kernel_start_p!(), kernel_end_p!());
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

/// # Safety
/// - `entry` must point to a physical location and be virtually mapped.
/// - `hid` must be equal to the executing hart ID.
/// - `dtb_ptr` must point to a physical location and be direct-mapped.
/// - `dyn_ptr` must point to the physical `_DYNAMIC` symbol and be direct-mapped.
pub unsafe fn jump_higher_half(entry: *const (), hid: usize, dtb_ptr: *const u8, dyn_ptr: *const rel::Elf64Dyn) -> ! {
    unsafe {
        let ventry = phys_to_kernel(PAddr::from_ptr(entry));
        let vdtb = phys_to_virt(PAddr::from_ptr(dtb_ptr));
        let vdyn = phys_to_kernel(PAddr::from_ptr(dyn_ptr));
        arch::jump_higher_half(ventry.as_ptr(), hid, vdtb, vdyn, phys_to_kernel(stack_end_p!()));
    }
}

pub struct VmsAccessGuard<'a, F>
where F : Fn(PAddr) -> VAddr {
    guard: MutexGuard<'a, Option<SendRootTable>, Spin>,
    p2v: &'a F,
    flush: bool,
}

impl<F : Fn(PAddr) -> VAddr> Drop for VmsAccessGuard<'_, F> {
    fn drop(&mut self) {
        if self.flush {
            arch::flush_vmap();
        }
    }
}

impl<F : Fn(PAddr) -> VAddr> VmsAccessGuard<'_, F> {
    pub fn root_pt_pa(&self) -> Result<PAddr, VmsError> {
        let wrapper = self.guard.as_ref().ok_or(VmsError::RootTableUninitialized)?;
        Ok(wrapper.root_pt_pa())
    }
    
    pub fn root_pt(&mut self) -> Result<&mut PageTable, VmsError> {
        let wrapper = self.guard.as_mut().ok_or(VmsError::RootTableUninitialized)?;
        Ok(unsafe { wrapper.root_pt(self.p2v) })
    }

    pub fn map_page(&mut self, va: VAddr, pa: PAddr, size: PageSize, flags: PTEFlags) -> Result<PageTableAllocMap, VmsError> {
        let wrapper = self.guard.as_mut().ok_or(VmsError::RootTableUninitialized)?;
        let root_pt = unsafe { wrapper.root_pt(self.p2v) };
        let allocs = arch::map_page(root_pt, va, pa, size, flags, self.p2v).map_err(|e| VmsError::Map(e))?;
        self.flush = true;
        Ok(allocs)
    }

    pub fn unmap_page(&mut self, va: VAddr) -> Result<PageSize, VmsError> {
        let wrapper = self.guard.as_mut().ok_or(VmsError::RootTableUninitialized)?;
        let root_pt = unsafe { wrapper.root_pt(self.p2v) };
        let unmapped_size = arch::unmap_page(root_pt, va, self.p2v).map_err(|e| VmsError::Unmap(e))?;
        self.flush = true;
        Ok(unmapped_size)
    }
}

pub fn acquire_with_p2v<F, T>(p2v: F, f: impl FnOnce(VmsAccessGuard<'_, F>) -> T) -> T
where F: Fn(PAddr) -> VAddr {
    let guard = ROOT_PT.lock();
    let guard = VmsAccessGuard { guard, p2v: &p2v, flush: false };
    f(guard)
}

pub fn acquire<T>(f: impl FnOnce(VmsAccessGuard<'_, fn(PAddr) -> VAddr>) -> T) -> T {
    let p2v: fn(PAddr) -> VAddr = phys_to_virt;
    acquire_with_p2v(p2v, f)
}
