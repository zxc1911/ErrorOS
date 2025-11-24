# 快速开始：第一阶段实现指南

本文档提供第一阶段（虚拟内存管理）的详细实现步骤，让你能够立即开始编码。

## 第 1 周：页表管理基础

### Day 1-2: 创建页表管理模块

#### 1. 创建文件结构
```bash
cd os/src
mkdir -p memory
touch memory/paging.rs
```

#### 2. 在 `memory/mod.rs` 中添加模块声明
```rust
// os/src/memory/mod.rs
pub mod paging;

// 重新导出常用类型
pub use paging::{PageTable, PageTableEntry, map_page, translate_addr};
```

#### 3. 实现页表遍历（`memory/paging.rs`）

```rust
use super::{PhysAddr, VirtAddr, PageTableFlags, PAGE_SIZE};

/// 遍历页表，将虚拟地址转换为物理地址
///
/// # 参数
/// - `root_paddr`: 根页表的物理地址
/// - `vaddr`: 要转换的虚拟地址
///
/// # 返回
/// - Some(PhysAddr): 转换成功
/// - None: 页面未映射
pub fn walk_page_table(root_paddr: PhysAddr, vaddr: VirtAddr) -> Option<PhysAddr> {
    // 获取根页表指针
    let root_table = unsafe {
        &*(root_paddr.as_usize() as *const super::PageTable)
    };

    // Level 2: VPN[2]
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry(vpn2);

    if !pte2.is_valid() {
        return None;  // 页面未映射
    }

    if pte2.is_leaf() {
        // Huge page (1GB)
        let offset = vaddr.as_usize() & 0x3FFF_FFFF;
        return Some(PhysAddr::new(pte2.phys_addr().as_usize() + offset));
    }

    // Level 1: VPN[1]
    let table1 = unsafe {
        &*(pte2.phys_addr().as_usize() as *const super::PageTable)
    };
    let vpn1 = vaddr.vpn1();
    let pte1 = table1.get_entry(vpn1);

    if !pte1.is_valid() {
        return None;
    }

    if pte1.is_leaf() {
        // Large page (2MB)
        let offset = vaddr.as_usize() & 0x1F_FFFF;
        return Some(PhysAddr::new(pte1.phys_addr().as_usize() + offset));
    }

    // Level 0: VPN[0]
    let table0 = unsafe {
        &*(pte1.phys_addr().as_usize() as *const super::PageTable)
    };
    let vpn0 = vaddr.vpn0();
    let pte0 = table0.get_entry(vpn0);

    if !pte0.is_valid() {
        return None;
    }

    // 4KB page
    let offset = vaddr.page_offset();
    Some(PhysAddr::new(pte0.phys_addr().as_usize() + offset))
}

/// 简化的地址转换（从当前页表）
pub fn translate_addr(vaddr: VirtAddr) -> Option<PhysAddr> {
    use riscv::register::satp;

    // 读取 satp 获取根页表地址
    let satp_value = satp::read();
    let root_ppn = satp_value.ppn();
    let root_paddr = PhysAddr::new(root_ppn << 12);

    walk_page_table(root_paddr, vaddr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_walk_page_table() {
        crate::serial_println!("[TEST] test_walk_page_table");

        // 测试恒等映射区域（内核代码）
        let kernel_vaddr = VirtAddr::new(0x8000_0000);
        let paddr = translate_addr(kernel_vaddr);

        // 早期启动时应该是恒等映射
        crate::serial_println!("Virtual: {:#x} -> Physical: {:#x?}",
            kernel_vaddr.as_usize(), paddr);
    }
}
```

### Day 3-4: 实现页面映射

#### 继续在 `memory/paging.rs` 中添加

```rust
use crate::memory::SimpleFrameAllocator;

/// 映射单个页面
///
/// # 参数
/// - `root_table`: 根页表（可变引用）
/// - `vaddr`: 虚拟地址
/// - `paddr`: 物理地址
/// - `flags`: 页表标志位
/// - `allocator`: 帧分配器（用于分配中间页表）
pub fn map_page(
    root_table: &mut super::PageTable,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: usize,
    allocator: &mut SimpleFrameAllocator,
) -> Result<(), &'static str> {
    // Level 2
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry_mut(vpn2);

    let table1 = if !pte2.is_valid() {
        // 分配新的二级页表
        let frame = allocator.allocate().ok_or("Out of memory")?;
        let table1_paddr = frame.start_address();

        // 设置页表项（指向下一级页表，V=1）
        pte2.set(table1_paddr.as_usize() >> 12, PageTableFlags::Valid as usize);

        // 清空新页表
        let table1 = unsafe {
            &mut *(table1_paddr.as_usize() as *mut super::PageTable)
        };
        table1.zero();
        table1
    } else {
        unsafe {
            &mut *(pte2.phys_addr().as_usize() as *mut super::PageTable)
        }
    };

    // Level 1
    let vpn1 = vaddr.vpn1();
    let pte1 = table1.get_entry_mut(vpn1);

    let table0 = if !pte1.is_valid() {
        let frame = allocator.allocate().ok_or("Out of memory")?;
        let table0_paddr = frame.start_address();

        pte1.set(table0_paddr.as_usize() >> 12, PageTableFlags::Valid as usize);

        let table0 = unsafe {
            &mut *(table0_paddr.as_usize() as *mut super::PageTable)
        };
        table0.zero();
        table0
    } else {
        unsafe {
            &mut *(pte1.phys_addr().as_usize() as *mut super::PageTable)
        }
    };

    // Level 0 - 设置最终映射
    let vpn0 = vaddr.vpn0();
    let pte0 = table0.get_entry_mut(vpn0);

    if pte0.is_valid() {
        return Err("Page already mapped");
    }

    // 设置叶子页表项
    pte0.set(paddr.as_usize() >> 12, flags | PageTableFlags::Valid as usize);

    // 刷新 TLB
    unsafe {
        riscv::asm::sfence_vma(Some(vaddr.as_usize()), None);
    }

    Ok(())
}

/// 取消页面映射
pub fn unmap_page(
    root_table: &mut super::PageTable,
    vaddr: VirtAddr,
) -> Result<PhysAddr, &'static str> {
    // 遍历页表找到最后一级
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry_mut(vpn2);

    if !pte2.is_valid() {
        return Err("Page not mapped");
    }

    let table1 = unsafe {
        &mut *(pte2.phys_addr().as_usize() as *mut super::PageTable)
    };

    let vpn1 = vaddr.vpn1();
    let pte1 = table1.get_entry_mut(vpn1);

    if !pte1.is_valid() {
        return Err("Page not mapped");
    }

    let table0 = unsafe {
        &mut *(pte1.phys_addr().as_usize() as *mut super::PageTable)
    };

    let vpn0 = vaddr.vpn0();
    let pte0 = table0.get_entry_mut(vpn0);

    if !pte0.is_valid() {
        return Err("Page not mapped");
    }

    let paddr = pte0.phys_addr();

    // 清除页表项
    *pte0 = super::PageTableEntry::new();

    // 刷新 TLB
    unsafe {
        riscv::asm::sfence_vma(Some(vaddr.as_usize()), None);
    }

    Ok(paddr)
}
```

### Day 5-7: 测试和调试

#### 在 `main.rs` 中添加测试代码

```rust
// os/src/main.rs 中的 kernel_main 函数
pub extern "C" fn kernel_main() -> ! {
    // ... 现有初始化代码 ...

    // 测试页表功能
    test_page_table(&mut memory_manager);

    // ... 继续执行 ...
}

fn test_page_table(memory_manager: &mut memory::MemoryManager) {
    use memory::{VirtAddr, PhysAddr, PageTableFlags, map_page, translate_addr};

    println!("\n=== Testing Page Table Mapping ===");

    // 创建测试页表
    let test_page_table_frame = memory_manager.frame_allocator
        .allocate()
        .expect("Failed to allocate page table");

    let test_page_table = unsafe {
        &mut *(test_page_table_frame.start_address().as_usize()
            as *mut memory::PageTable)
    };
    test_page_table.zero();

    // 测试映射
    let vaddr = VirtAddr::new(0x1000_0000);
    let paddr = PhysAddr::new(0x8100_0000);

    println!("Mapping {:?} -> {:?}", vaddr, paddr);

    let flags = PageTableFlags::Read as usize
        | PageTableFlags::Write as usize
        | PageTableFlags::Valid as usize;

    match map_page(
        test_page_table,
        vaddr,
        paddr,
        flags,
        &mut memory_manager.frame_allocator
    ) {
        Ok(_) => println!("✓ Mapping successful"),
        Err(e) => println!("✗ Mapping failed: {}", e),
    }

    // 注意：此时还没有切换页表，所以 translate_addr 看不到新映射
    println!("=== Page Table Test Complete ===\n");
}
```

---

## 第 2 周：地址空间抽象

### Day 8-10: 实现地址空间结构

#### 1. 创建 `memory/address_space.rs`

```rust
use super::{PageTable, PhysAddr, VirtAddr, SimpleFrameAllocator, PAGE_SIZE};
use alloc::vec::Vec;
use core::ops::Range;

/// 内存区域类型
#[derive(Debug, Clone, Copy)]
pub enum MemoryAreaType {
    Code,      // 代码段（R-X）
    Data,      // 数据段（RW-）
    Heap,      // 堆（RW-）
    Stack,     // 栈（RW-）
    Shared,    // 共享内存（RW-）
}

/// 内存区域
#[derive(Debug, Clone)]
pub struct MemoryArea {
    pub range: Range<VirtAddr>,
    pub area_type: MemoryAreaType,
    pub flags: usize,
}

/// 地址空间
pub struct AddressSpace {
    page_table: Box<PageTable>,
    areas: Vec<MemoryArea>,
}

impl AddressSpace {
    /// 创建新的地址空间
    pub fn new(allocator: &mut SimpleFrameAllocator) -> Result<Self, &'static str> {
        // 分配根页表
        let frame = allocator.allocate().ok_or("Out of memory")?;
        let page_table_ptr = frame.start_address().as_usize() as *mut PageTable;

        let mut page_table = unsafe { Box::from_raw(page_table_ptr) };
        page_table.zero();

        Ok(AddressSpace {
            page_table,
            areas: Vec::new(),
        })
    }

    /// 映射内存区域
    pub fn map_region(
        &mut self,
        start: VirtAddr,
        size: usize,
        area_type: MemoryAreaType,
        flags: usize,
        allocator: &mut SimpleFrameAllocator,
    ) -> Result<(), &'static str> {
        use super::paging::map_page;

        let end = VirtAddr::new(start.as_usize() + size);
        let area = MemoryArea {
            range: start..end,
            area_type,
            flags,
        };

        // 分配并映射每个页面
        let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..page_count {
            let vaddr = VirtAddr::new(start.as_usize() + i * PAGE_SIZE);
            let frame = allocator.allocate().ok_or("Out of memory")?;
            let paddr = frame.start_address();

            map_page(&mut self.page_table, vaddr, paddr, flags, allocator)?;
        }

        self.areas.push(area);
        Ok(())
    }

    /// 激活此地址空间（写入 satp）
    pub fn activate(&self) {
        use riscv::register::satp;

        let page_table_paddr = &*self.page_table as *const _ as usize;
        let ppn = page_table_paddr >> 12;

        unsafe {
            // Sv39 模式，ASID = 0
            satp::set(satp::Mode::Sv39, 0, ppn);
            // 刷新整个 TLB
            riscv::asm::sfence_vma_all();
        }

        crate::serial_println!("[MEMORY] Activated address space at PPN {:#x}", ppn);
    }

    /// 获取页表的物理地址
    pub fn page_table_paddr(&self) -> PhysAddr {
        let ptr = &*self.page_table as *const _ as usize;
        PhysAddr::new(ptr)
    }
}
```

### Day 11-14: 实现内核地址空间

#### 在 `memory/mod.rs` 中添加

```rust
pub mod address_space;
pub use address_space::{AddressSpace, MemoryAreaType};

/// 创建内核地址空间
///
/// # 功能
/// - 映射内核代码和数据段
/// - 映射物理内存（恒等映射）
/// - 映射 MMIO 设备
pub fn create_kernel_address_space(
    allocator: &mut SimpleFrameAllocator
) -> Result<AddressSpace, &'static str> {
    let mut addr_space = AddressSpace::new(allocator)?;

    // 1. 恒等映射内核区域（0x80000000 - 0x88000000）
    const KERNEL_START: usize = 0x8000_0000;
    const KERNEL_SIZE: usize = 128 * 1024 * 1024; // 128 MB

    use PageTableFlags as PTF;
    let kernel_flags = PTF::Valid as usize
        | PTF::Read as usize
        | PTF::Write as usize
        | PTF::Execute as usize;

    crate::serial_println!("[MEMORY] Mapping kernel region: {:#x} - {:#x}",
        KERNEL_START, KERNEL_START + KERNEL_SIZE);

    // 映射整个内核区域（使用恒等映射）
    for offset in (0..KERNEL_SIZE).step_by(PAGE_SIZE) {
        let addr = KERNEL_START + offset;
        let vaddr = VirtAddr::new(addr);
        let paddr = PhysAddr::new(addr);

        paging::map_page(&mut addr_space.page_table, vaddr, paddr, kernel_flags, allocator)?;
    }

    // 2. 映射 UART（0x10000000）
    const UART_BASE: usize = 0x1000_0000;
    let io_flags = PTF::Valid as usize | PTF::Read as usize | PTF::Write as usize;

    paging::map_page(
        &mut addr_space.page_table,
        VirtAddr::new(UART_BASE),
        PhysAddr::new(UART_BASE),
        io_flags,
        allocator
    )?;

    Ok(addr_space)
}
```

#### 在 `main.rs` 中启用虚拟内存

```rust
pub extern "C" fn kernel_main() -> ! {
    // ... 现有初始化 ...

    // 创建并激活内核地址空间
    println!("\n=== Enabling Virtual Memory ===");

    let kernel_addr_space = memory::create_kernel_address_space(
        &mut memory_manager.frame_allocator
    ).expect("Failed to create kernel address space");

    println!("Activating kernel address space...");
    kernel_addr_space.activate();

    println!("✓ Virtual memory enabled!");

    // 继续执行...
}
```

---

## 第 3 周：完善和测试

### Day 15-18: 添加完整测试

```rust
// os/src/memory/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_full_memory_system() {
        serial_println!("[TEST] Full memory system test");

        extern "C" {
            static kernel_end: u8;
        }
        let kernel_end_addr = unsafe { &kernel_end as *const u8 as usize };

        let mut memory_manager = init(kernel_end_addr);

        // 测试页面映射
        let mut addr_space = AddressSpace::new(&mut memory_manager.frame_allocator)
            .expect("Failed to create address space");

        let vaddr = VirtAddr::new(0x1000_0000);
        let frame = memory_manager.frame_allocator.allocate()
            .expect("Failed to allocate frame");

        let flags = PageTableFlags::Read as usize
            | PageTableFlags::Write as usize
            | PageTableFlags::Valid as usize;

        paging::map_page(
            &mut addr_space.page_table,
            vaddr,
            frame.start_address(),
            flags,
            &mut memory_manager.frame_allocator
        ).expect("Failed to map page");

        serial_println!("[TEST] ✓ Page mapping successful");
    }
}
```

### Day 19-21: 性能优化和文档

1. 添加 TLB 刷新优化
2. 实现批量映射
3. 编写详细的文档注释

---

## 常见问题和解决方案

### Q1: 启用虚拟内存后系统崩溃
**原因**：可能没有正确映射内核代码段或栈

**解决**：确保恒等映射覆盖了所有内核使用的内存区域

```rust
// 检查当前代码地址
let current_pc: usize;
unsafe {
    asm!("auipc {}, 0", out(reg) current_pc);
}
println!("Current PC: {:#x}", current_pc);
```

### Q2: 页面映射失败
**原因**：可能是标志位设置不正确

**解决**：确保至少设置了 Valid 位
```rust
let flags = PageTableFlags::Valid as usize | /* 其他标志 */;
```

### Q3: 无法访问 MMIO 设备
**原因**：忘记映射设备地址

**解决**：添加 UART 等设备的映射
```rust
const UART_BASE: usize = 0x1000_0000;
map_page(/* ... */, VirtAddr::new(UART_BASE), PhysAddr::new(UART_BASE), /* ... */);
```

---

## 检查清单

完成第一阶段后，你应该能够：

- [ ] 实现页表遍历（3 级）
- [ ] 实现页面映射和取消映射
- [ ] 创建独立的地址空间
- [ ] 激活内核地址空间
- [ ] 系统能在虚拟内存模式下稳定运行
- [ ] 所有现有功能（串口、中断等）正常工作

---

## 下一步

完成第一阶段后，参考主路线图文档进入第二阶段：系统调用机制。

祝开发顺利！如有问题，参考 xv6-riscv 或 rCore 的实现。
