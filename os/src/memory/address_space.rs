/*
 * ============================================
 * RISC-V 地址空间抽象
 * ============================================
 * 功能：管理独立的虚拟地址空间
 *
 * 教学说明：
 * - 每个进程都有独立的地址空间
 * - 地址空间包含页表和内存区域
 * - 可以激活（切换到）某个地址空间
 * ============================================
 */

use super::{PageTable, PhysAddr, VirtAddr, PageTableFlags, SimpleFrameAllocator, PAGE_SIZE};
use super::paging::{map_page, unmap_page};
use alloc::vec::Vec;
use core::ops::Range;

/// 内存区域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAreaType {
    Code,      // 代码段（R-X）
    Data,      // 数据段（RW-）
    Heap,      // 堆（RW-）
    Stack,     // 栈（RW-）
    Shared,    // 共享内存（RW-）
}

impl MemoryAreaType {
    /// 获取该区域类型的默认标志位
    pub fn default_flags(&self) -> usize {
        use PageTableFlags as PTF;

        match self {
            MemoryAreaType::Code => {
                // 代码段：可读、可执行
                PTF::Valid as usize | PTF::Read as usize | PTF::Execute as usize
            }
            MemoryAreaType::Data | MemoryAreaType::Heap | MemoryAreaType::Stack => {
                // 数据段、堆、栈：可读、可写
                PTF::Valid as usize | PTF::Read as usize | PTF::Write as usize
            }
            MemoryAreaType::Shared => {
                // 共享内存：可读、可写
                PTF::Valid as usize | PTF::Read as usize | PTF::Write as usize
            }
        }
    }
}

/// 内存区域
///
/// # 教学说明
/// 内存区域是地址空间的一部分，具有连续的虚拟地址和相同的权限
#[derive(Debug, Clone)]
pub struct MemoryArea {
    pub range: Range<VirtAddr>,
    pub area_type: MemoryAreaType,
    pub flags: usize,
}

impl MemoryArea {
    /// 创建新的内存区域
    pub fn new(start: VirtAddr, end: VirtAddr, area_type: MemoryAreaType) -> Self {
        MemoryArea {
            range: start..end,
            area_type,
            flags: area_type.default_flags(),
        }
    }

    /// 获取区域大小
    pub fn size(&self) -> usize {
        self.range.end.as_usize() - self.range.start.as_usize()
    }

    /// 获取页数
    pub fn page_count(&self) -> usize {
        (self.size() + PAGE_SIZE - 1) / PAGE_SIZE
    }
}

/// 地址空间
///
/// # 教学说明
/// 地址空间代表一个独立的虚拟内存空间，包含：
/// - 一个根页表
/// - 多个内存区域
/// - 可以激活（切换satp寄存器）
pub struct AddressSpace {
    page_table: *mut PageTable,
    page_table_paddr: PhysAddr,
    areas: Vec<MemoryArea>,
}

impl AddressSpace {
    /// 创建新的地址空间
    ///
    /// # 教学说明
    /// 1. 分配一个物理帧作为根页表
    /// 2. 清空页表
    /// 3. 初始化空的内存区域列表
    pub fn new(allocator: &mut SimpleFrameAllocator) -> Result<Self, &'static str> {
        // 分配根页表
        let frame = allocator.allocate().ok_or("Out of memory")?;
        let page_table_paddr = frame.start_address();
        let page_table_ptr = page_table_paddr.as_usize() as *mut PageTable;

        // 清空页表
        unsafe {
            (*page_table_ptr).zero();
        }

        crate::serial_println!(
            "[ADDRESS_SPACE] Created new address space, page table at {:#x}",
            page_table_paddr.as_usize()
        );

        Ok(AddressSpace {
            page_table: page_table_ptr,
            page_table_paddr,
            areas: Vec::new(),
        })
    }

    /// 映射内存区域
    ///
    /// # 参数
    /// - `start`: 起始虚拟地址
    /// - `size`: 区域大小
    /// - `area_type`: 区域类型
    /// - `allocator`: 帧分配器
    ///
    /// # 教学说明
    /// 1. 创建内存区域描述
    /// 2. 分配物理帧
    /// 3. 建立虚拟地址到物理地址的映射
    pub fn map_region(
        &mut self,
        start: VirtAddr,
        size: usize,
        area_type: MemoryAreaType,
        allocator: &mut SimpleFrameAllocator,
    ) -> Result<(), &'static str> {
        let end = VirtAddr::new(start.as_usize() + size);
        let area = MemoryArea::new(start, end, area_type);

        crate::serial_println!(
            "[ADDRESS_SPACE] Mapping region: {:#x} - {:#x} ({:?})",
            start.as_usize(),
            end.as_usize(),
            area_type
        );

        // 分配并映射每个页面
        let page_count = area.page_count();

        for i in 0..page_count {
            let vaddr = VirtAddr::new(start.as_usize() + i * PAGE_SIZE);

            // 分配物理帧
            let frame = allocator.allocate().ok_or("Out of memory")?;
            let paddr = frame.start_address();

            // 建立映射
            unsafe {
                map_page(&mut *self.page_table, vaddr, paddr, area.flags, allocator)?;
            }
        }

        self.areas.push(area);
        Ok(())
    }

    /// 映射内存区域（恒等映射）
    ///
    /// # 教学说明
    /// 恒等映射：虚拟地址 == 物理地址
    /// 主要用于内核代码段，方便直接访问物理内存
    pub fn map_region_identity(
        &mut self,
        start: PhysAddr,
        size: usize,
        area_type: MemoryAreaType,
        allocator: &mut SimpleFrameAllocator,
    ) -> Result<(), &'static str> {
        let vstart = VirtAddr::new(start.as_usize());
        let end = VirtAddr::new(start.as_usize() + size);
        let area = MemoryArea::new(vstart, end, area_type);

        crate::serial_println!(
            "[ADDRESS_SPACE] Identity mapping region: {:#x} - {:#x} ({:?})",
            start.as_usize(),
            start.as_usize() + size,
            area_type
        );

        // 映射每个页面（恒等映射）
        let page_count = area.page_count();

        for i in 0..page_count {
            let addr = start.as_usize() + i * PAGE_SIZE;
            let vaddr = VirtAddr::new(addr);
            let paddr = PhysAddr::new(addr);

            // 建立恒等映射
            unsafe {
                map_page(&mut *self.page_table, vaddr, paddr, area.flags, allocator)?;
            }
        }

        self.areas.push(area);
        Ok(())
    }

    /// 取消映射内存区域
    pub fn unmap_region(&mut self, start: VirtAddr, size: usize) -> Result<(), &'static str> {
        let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..page_count {
            let vaddr = VirtAddr::new(start.as_usize() + i * PAGE_SIZE);
            unsafe {
                unmap_page(&mut *self.page_table, vaddr)?;
            }
        }

        // 从区域列表中移除
        self.areas.retain(|area| {
            !(area.range.start.as_usize() == start.as_usize())
        });

        Ok(())
    }

    /// 激活此地址空间（写入 satp）
    ///
    /// # 教学说明
    /// 1. 计算页表的物理页号（PPN）
    /// 2. 设置 satp 寄存器（Sv39 模式）
    /// 3. 刷新 TLB
    pub fn activate(&self) {
        use riscv::register::satp;

        let ppn = self.page_table_paddr.as_usize() >> 12;

        crate::serial_println!(
            "[ADDRESS_SPACE] Activating address space, PPN: {:#x}",
            ppn
        );

        unsafe {
            // Sv39 模式，ASID = 0
            satp::set(satp::Mode::Sv39, 0, ppn);

            // 刷新整个 TLB
            core::arch::asm!("sfence.vma");
        }

        crate::serial_println!("[ADDRESS_SPACE] Address space activated");
    }

    /// 获取页表的物理地址
    pub fn page_table_paddr(&self) -> PhysAddr {
        self.page_table_paddr
    }

    /// 获取内存区域列表
    pub fn areas(&self) -> &[MemoryArea] {
        &self.areas
    }

    /// 可视化显示地址空间布局（教学特色）
    pub fn print_layout(&self) {
        crate::serial_println!("\n╔════════════════════════════════════════╗");
        crate::serial_println!("║      地址空间布局                      ║");
        crate::serial_println!("╠════════════════════════════════════════╣");
        crate::serial_println!("║ 页表物理地址: {:#018x}      ║", self.page_table_paddr.as_usize());
        crate::serial_println!("╠════════════════════════════════════════╣");
        crate::serial_println!("║ 内存区域:                              ║");

        for (i, area) in self.areas.iter().enumerate() {
            crate::serial_println!(
                "║ [{:2}] {:8?} {:#010x} - {:#010x} ║",
                i,
                area.area_type,
                area.range.start.as_usize(),
                area.range.end.as_usize()
            );
        }

        crate::serial_println!("╚════════════════════════════════════════╝\n");
    }
}

// 由于我们存储的是原始指针，需要手动实现 Send
unsafe impl Send for AddressSpace {}
unsafe impl Sync for AddressSpace {}

/// 创建内核地址空间
///
/// # 功能
/// - 映射内核代码和数据段（恒等映射）
/// - 映射物理内存
/// - 映射 MMIO 设备
///
/// # 教学说明
/// 内核地址空间使用恒等映射，这样内核代码可以直接访问物理地址
pub fn create_kernel_address_space(
    allocator: &mut SimpleFrameAllocator
) -> Result<AddressSpace, &'static str> {
    let mut addr_space = AddressSpace::new(allocator)?;

    crate::serial_println!("\n[KERNEL] Creating kernel address space...");

    // 1. 恒等映射内核区域（0x80000000 - 0x88000000，128 MB）
    const KERNEL_START: usize = 0x8000_0000;
    const KERNEL_SIZE: usize = 128 * 1024 * 1024; // 128 MB

    crate::serial_println!(
        "[KERNEL] Mapping kernel region: {:#x} - {:#x}",
        KERNEL_START,
        KERNEL_START + KERNEL_SIZE
    );

    // 分段映射，避免一次性分配太多页表
    // 先映射前 16MB（包含内核代码）
    addr_space.map_region_identity(
        PhysAddr::new(KERNEL_START),
        16 * 1024 * 1024,  // 16 MB
        MemoryAreaType::Code,
        allocator
    )?;

    // 2. 映射 UART（0x10000000）
    const UART_BASE: usize = 0x1000_0000;

    crate::serial_println!("[KERNEL] Mapping UART: {:#x}", UART_BASE);

    addr_space.map_region_identity(
        PhysAddr::new(UART_BASE),
        PAGE_SIZE,
        MemoryAreaType::Data,
        allocator
    )?;

    crate::serial_println!("[KERNEL] Kernel address space created successfully\n");

    Ok(addr_space)
}
