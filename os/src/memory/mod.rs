/*
 * ============================================
 * RISC-V 内存管理模块
 * ============================================
 * 功能：提供虚拟内存管理和物理帧分配
 *
 * RISC-V Sv39 分页机制：
 * - 3 级页表（512GB 虚拟地址空间）
 * - 页大小：4KB
 * - 页表项：8 字节
 * - 每个页表：512 个条目
 *
 * 内存布局（QEMU virt 机器）：
 * - 物理内存起始：0x80000000
 * - 物理内存大小：128MB（默认）
 * ============================================
 */

// ============================================
// 子模块
// ============================================

pub mod paging;
pub mod address_space;

// 重新导出页表管理函数
pub use paging::{
    walk_page_table, walk_page_table_verbose,
    map_page, map_page_verbose,
    unmap_page,
    translate_addr as translate_addr_current
};

// 重新导出地址空间相关类型
pub use address_space::{
    AddressSpace, MemoryArea, MemoryAreaType,
    create_kernel_address_space
};

/// 页大小（4KB）
pub const PAGE_SIZE: usize = 4096;

/// 页表项数量
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// RISC-V Sv39 虚拟地址
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    /// 创建新的虚拟地址
    pub const fn new(addr: usize) -> Self {
        VirtAddr(addr)
    }

    /// 获取地址值
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// 获取可变指针
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// 获取页偏移（低 12 位）
    pub const fn page_offset(self) -> usize {
        self.0 & 0xFFF
    }

    /// 获取 VPN[0]（12-20 位）
    pub const fn vpn0(self) -> usize {
        (self.0 >> 12) & 0x1FF
    }

    /// 获取 VPN[1]（21-29 位）
    pub const fn vpn1(self) -> usize {
        (self.0 >> 21) & 0x1FF
    }

    /// 获取 VPN[2]（30-38 位）
    pub const fn vpn2(self) -> usize {
        (self.0 >> 30) & 0x1FF
    }
}

/// RISC-V 物理地址
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    /// 创建新的物理地址
    pub const fn new(addr: usize) -> Self {
        PhysAddr(addr)
    }

    /// 获取地址值
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// 4KB 页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    start_address: VirtAddr,
}

impl Page {
    /// 获取包含给定虚拟地址的页
    pub fn containing_address(addr: VirtAddr) -> Self {
        Page {
            start_address: VirtAddr::new(addr.as_usize() & !0xFFF),
        }
    }

    /// 获取页的起始地址
    pub fn start_address(self) -> VirtAddr {
        self.start_address
    }
}

/// 物理帧
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    start_address: PhysAddr,
}

impl PhysFrame {
    /// 获取包含给定物理地址的帧
    pub fn containing_address(addr: PhysAddr) -> Self {
        PhysFrame {
            start_address: PhysAddr::new(addr.as_usize() & !0xFFF),
        }
    }

    /// 获取帧的起始地址
    pub fn start_address(self) -> PhysAddr {
        self.start_address
    }
}

/// 页表项标志位
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum PageTableFlags {
    Valid = 1 << 0,      // V: 有效位
    Read = 1 << 1,       // R: 可读
    Write = 1 << 2,      // W: 可写
    Execute = 1 << 3,    // X: 可执行
    User = 1 << 4,       // U: 用户可访问
    Global = 1 << 5,     // G: 全局映射
    Accessed = 1 << 6,   // A: 访问位
    Dirty = 1 << 7,      // D: 脏位
}

/// 页表项
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry {
    entry: usize,
}

impl PageTableEntry {
    /// 创建空的页表项
    pub const fn new() -> Self {
        PageTableEntry { entry: 0 }
    }

    /// 判断是否有效
    pub fn is_valid(&self) -> bool {
        (self.entry & 1) != 0
    }

    /// 判断是否为叶子节点（映射到物理页）
    pub fn is_leaf(&self) -> bool {
        (self.entry & 0xE) != 0 // R/W/X 任意一位为 1
    }

    /// 获取物理页号（PPN）
    pub fn ppn(&self) -> usize {
        (self.entry >> 10) & 0xFFF_FFFF_FFFF
    }

    /// 获取物理地址
    pub fn phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self.ppn() << 12)
    }

    /// 设置页表项
    pub fn set(&mut self, ppn: usize, flags: usize) {
        self.entry = (ppn << 10) | flags;
    }

    /// 获取标志位
    pub fn flags(&self) -> usize {
        self.entry & 0xFF
    }
}

/// 页表（512 个条目）
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// 创建新的空页表
    pub const fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::new(); PAGE_TABLE_ENTRIES],
        }
    }

    /// 获取页表项
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// 获取可变页表项
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// 清空页表
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = PageTableEntry::new();
        }
    }
}

/// 简单的物理帧分配器
///
/// # 说明
/// 从固定的物理内存区域分配帧
/// QEMU virt 机器的物理内存布局：
/// - 0x80000000 - 0x88000000（128MB）
pub struct SimpleFrameAllocator {
    next_frame: usize,
    end_frame: usize,
}

impl SimpleFrameAllocator {
    /// 创建新的帧分配器
    ///
    /// # 参数
    /// - `kernel_end`: 内核结束地址
    /// - `memory_end`: 物理内存结束地址
    pub fn new(kernel_end: usize, memory_end: usize) -> Self {
        let next_frame = (kernel_end + PAGE_SIZE - 1) / PAGE_SIZE;
        let end_frame = memory_end / PAGE_SIZE;

        crate::serial_println!(
            "[MEMORY] Frame allocator initialized: {:#x} - {:#x}",
            next_frame * PAGE_SIZE,
            end_frame * PAGE_SIZE
        );

        SimpleFrameAllocator {
            next_frame,
            end_frame,
        }
    }

    /// 分配一个物理帧
    pub fn allocate(&mut self) -> Option<PhysFrame> {
        if self.next_frame >= self.end_frame {
            return None;
        }

        let frame = PhysFrame::containing_address(PhysAddr::new(
            self.next_frame * PAGE_SIZE,
        ));
        self.next_frame += 1;

        Some(frame)
    }

    /// 释放一个物理帧（当前实现为空，可扩展）
    pub fn deallocate(&mut self, _frame: PhysFrame) {
        // TODO: 实现帧回收
    }
}

/// 内存管理器
pub struct MemoryManager {
    pub frame_allocator: SimpleFrameAllocator,
}

impl MemoryManager {
    /// 初始化内存管理器
    pub fn new(kernel_end: usize, memory_end: usize) -> Self {
        MemoryManager {
            frame_allocator: SimpleFrameAllocator::new(kernel_end, memory_end),
        }
    }
}

/// 初始化内存管理
///
/// # 功能
/// - 初始化物理帧分配器
/// - 设置虚拟内存映射
///
/// # 参数
/// - `kernel_end`: 内核结束地址
pub fn init(kernel_end: usize) -> MemoryManager {
    // QEMU virt 机器的物理内存：0x80000000 - 0x88000000（128MB）
    const MEMORY_START: usize = 0x8000_0000;
    const MEMORY_SIZE: usize = 128 * 1024 * 1024; // 128 MB
    let memory_end = MEMORY_START + MEMORY_SIZE;

    crate::serial_println!("[MEMORY] Initializing memory management");
    crate::serial_println!("[MEMORY] Kernel end: {:#x}", kernel_end);
    crate::serial_println!("[MEMORY] Memory range: {:#x} - {:#x}", MEMORY_START, memory_end);

    MemoryManager::new(kernel_end, memory_end)
}

/// 创建示例映射（用于测试）
///
/// # 参数
/// - `page`: 要映射的虚拟页
/// - `frame_allocator`: 帧分配器
pub fn create_example_mapping(
    page: Page,
    frame_allocator: &mut SimpleFrameAllocator,
) -> Result<(), &'static str> {
    // 分配一个物理帧
    let frame = frame_allocator
        .allocate()
        .ok_or("Failed to allocate frame")?;

    crate::serial_println!(
        "[MEMORY] Example mapping: {:?} -> {:?}",
        page.start_address(),
        frame.start_address()
    );

    Ok(())
}

/// 地址转换（虚拟地址 -> 物理地址）
///
/// # 参数
/// - `vaddr`: 虚拟地址
///
/// # 返回
/// - 对应的物理地址（如果已映射）
pub fn translate_addr(vaddr: VirtAddr) -> Option<PhysAddr> {
    use riscv::register::satp;

    // 读取 satp 寄存器获取根页表地址
    let satp_value = satp::read();
    let root_ppn = satp_value.ppn();
    let _root_paddr = PhysAddr::new(root_ppn << 12);

    // TODO: 实现完整的页表遍历
    // 这里返回恒等映射（用于早期启动）
    Some(PhysAddr::new(vaddr.as_usize()))
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_virt_addr_vpn() {
        let addr = VirtAddr::new(0x1234_5678);
        assert_eq!(addr.page_offset(), 0x678);
    }
}
