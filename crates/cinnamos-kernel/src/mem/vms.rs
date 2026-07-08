use core::mem::ManuallyDrop;

use fdt::Fdt;
use spin::{Mutex, MutexGuard, Spin};

use crate::{arch::{
    self, DIRECT_MAP_BASE, MapError, PAddr, PTEFlags, PageSize, PageTable, PageTableAllocMap, UnmapError, VAddr,
}, mem::{FrameAlloc, PAGE_SIZE, palloc::{self, Alloc}}, println};

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

    unsafe fn root_pt(&mut self, p2v: impl Fn(PAddr) -> VAddr) -> &mut PageTable {
        unsafe { p2v(self.0.base_addr()).as_mut::<PageTable>().as_mut_unchecked() }
    }
}

unsafe impl Send for SendRootTable {}

pub struct VirtualMemoryInfo {
    pub max_asid: usize,
}

static ROOT_PT: Mutex<Option<SendRootTable>> = Mutex::new(None);

unsafe extern "C" {
    static KERNEL_START: PAddr;
    static KERNEL_END: PAddr;
    
    static TEXT_START: PAddr;
    static TEXT_END: PAddr;
    
    static DATA_START: PAddr;
    static DATA_END: PAddr;
    
    static BSS_START: PAddr;
    static BSS_END: PAddr;
    
    static KMEM_START: PAddr;
    static KMEM_END: PAddr;

    static STACK_END: PAddr;
}

macro_rules! kernel_map_offset {
    () => ({
        use crate::arch::{KERNEL_MAP_BASE, PAddr};
        unsafe extern "C" { static KERNEL_START: PAddr; }
        unsafe { KERNEL_MAP_BASE - KERNEL_START.addr() }
    });
}

pub fn phys_to_kernel(pa: PAddr) -> VAddr {
    VAddr::new(pa.addr() + kernel_map_offset!())
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

pub unsafe fn init_kernel_map(fdt: &Fdt) -> Result<VirtualMemoryInfo, VmsError> {
    acquire_with_p2v(&VAddr::identity, |mut g| {
        unsafe {
            println!("id-mapping text\t: 0x{:016x} .. 0x{:016x}", TEXT_START, TEXT_END);
            let mut pa = TEXT_START;
            while pa < TEXT_END {                
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, TEXT_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RX)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-mapping data\t: 0x{:016x} .. 0x{:016x}", DATA_START, DATA_END);
            pa = DATA_START;
            while pa < DATA_END {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, DATA_START - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::READ)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-mapping bss\t: 0x{:016x} .. 0x{:016x}", BSS_START, BSS_END);
            pa = BSS_START;
            while pa < BSS_END {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, BSS_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }
            println!("id-mapping kmem\t: 0x{:016x} .. 0x{:016x}", KMEM_START, KMEM_END);
            pa = KMEM_START;
            while pa < KMEM_END {
                let va = VAddr::identity(pa);
                let next_size = PageSize::select_size(va, pa, KMEM_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }

            println!("hi-mapping text\t: 0x{:016x} .. 0x{:016x}", TEXT_START, TEXT_END);
            pa = TEXT_START;
            while pa < TEXT_END {                
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, TEXT_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RX)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-mapping data\t: 0x{:016x} .. 0x{:016x}", DATA_START, DATA_END);
            pa = DATA_START;
            while pa < DATA_END {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, DATA_START - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::READ)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-mapping bss\t: 0x{:016x} .. 0x{:016x}", BSS_START, BSS_END);
            pa = BSS_START;
            while pa < BSS_END {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, BSS_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }
            println!("hi-mapping kmem\t: 0x{:016x} .. 0x{:016x}", KMEM_START, KMEM_END);
            pa = KMEM_START;
            while pa < KMEM_END {
                let va = phys_to_kernel(pa);
                let next_size = PageSize::select_size(va, pa, KMEM_END - pa).ok_or(VmsError::Unaligned)?;
                g.map_page(va, pa, next_size, PTEFlags::RW)?.forget();
                pa = pa + next_size.size();
            }
    
            for r in fdt.memory().regions() {
                if let Some(size) = r.size {
                    pa = PAddr::from_ptr(r.starting_address);
                    let pa_end = pa + size;

                    println!("mapping mem\t: 0x{:016x} .. 0x{:016x}", pa, pa_end);
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

                                println!("mapping /soc/{}\t: 0x{:016x} .. 0x{:016x}", n.name, pa, pa_end);
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

            #[cfg(debug_assertions)]
            {
                let mut pa_orig = KERNEL_START;
                while pa_orig < KERNEL_END {
                    let va = phys_to_kernel(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Phys-to-kernel translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
                    
                    pa_orig = pa_orig + PAGE_SIZE;
                }

                pa_orig = KERNEL_START;
                while pa_orig < KERNEL_END {
                    let va = phys_to_virt(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Phys-to-direct translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
    
                    pa_orig = pa_orig + PAGE_SIZE;
                }
                
                pa_orig = KERNEL_START;
                while pa_orig < KERNEL_END {
                    let va = VAddr::identity(pa_orig);
                    let pa_trns = arch::translate_virt(g.root_pt()?, va, VAddr::identity);
                    let va_addr = va.addr();
                    let pa_orig_addr = pa_orig.addr();
                    let pa_trns_addr = pa_trns.unwrap_or(PAddr::new(0)).addr();
                    debug_assert_eq!(pa_trns, Some(pa_orig), "Identity-vtmap translation failed 0x{va_addr:016x} -> 0x{pa_trns_addr:016x} vs. 0x{pa_orig_addr:016x}");
        
                    pa_orig = pa_orig + PAGE_SIZE;
                }
            }

            let root_pt_pa = g.root_pt_pa()?;
            let max_asid = arch::activate_vmap(root_pt_pa);
            Ok(VirtualMemoryInfo { max_asid })
        }
    })
}

pub unsafe fn jump_higher_half(entry: *const (), hid: usize, dtb_ptr: *const u8) -> ! {
    unsafe {
        let ventry = phys_to_kernel(PAddr::from_ptr(entry));
        let vdtb = phys_to_virt(PAddr::from_ptr(dtb_ptr));
        arch::jump_to_higher_half(ventry.as_ptr(), hid, vdtb, phys_to_kernel(STACK_END));
    }
}

pub struct VmsAccessGuard<'a, F>
where F : Fn(PAddr) -> VAddr {
    guard: MutexGuard<'a, Option<SendRootTable>, Spin>,
    p2v: &'a F,
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
        arch::map_page(root_pt, va, pa, size, flags, self.p2v).map_err(|e| VmsError::Map(e))
    }

    pub fn unmap_page(&mut self, va: VAddr) -> Result<(), VmsError> {
        let wrapper = self.guard.as_mut().ok_or(VmsError::RootTableUninitialized)?;
        let root_pt = unsafe { wrapper.root_pt(self.p2v) };
        arch::unmap_page(root_pt, va, self.p2v).map_err(|e| VmsError::Unmap(e))
    }
}

pub fn acquire_with_p2v<F, T>(p2v: F, f: impl FnOnce(VmsAccessGuard<'_, F>) -> T) -> T
where F: Fn(PAddr) -> VAddr {
    let guard = ROOT_PT.lock();
    let guard = VmsAccessGuard { guard, p2v: &p2v };
    f(guard)
}

pub fn acquire<T>(f: impl FnOnce(VmsAccessGuard<'_, fn(PAddr) -> VAddr>) -> T) -> T {
    let p2v: fn(PAddr) -> VAddr = phys_to_virt;
    acquire_with_p2v(p2v, f)
}
