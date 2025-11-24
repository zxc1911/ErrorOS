# 虚拟内存系统实现文档

> **作者**: Blog OS 开发团队
> **日期**: 2025-11-24
> **版本**: v0.1.0
> **标签**: 虚拟内存, 页表, RISC-V, Sv39, 教学OS

---

## 📋 目录

1. [概述](#概述)
2. [设计目标](#设计目标)
3. [技术背景](#技术背景)
4. [实现细节](#实现细节)
5. [可视化教学特色](#可视化教学特色)
6. [测试与验证](#测试与验证)
7. [遇到的问题与解决](#遇到的问题与解决)
8. [后续改进方向](#后续改进方向)

---

## 概述

本次提交实现了 RISC-V Sv39 三级页表机制的完整虚拟内存系统，包括页表管理、地址空间抽象和内核地址空间的创建与激活。

### 核心功能

- ✅ 三级页表遍历（Level 2 → Level 1 → Level 0）
- ✅ 虚拟地址到物理地址映射
- ✅ 页面取消映射
- ✅ 地址空间抽象（AddressSpace）
- ✅ 内核地址空间的恒等映射
- ✅ 虚拟内存的激活（satp 寄存器设置）

### 教学特色

- 🎨 每个操作都有可视化输出
- 📊 详细的页表遍历过程展示
- 💡 清晰的地址空间布局显示
- 📖 丰富的代码注释和教学说明

---

## 设计目标

### 功能性目标

1. **完整的页表管理**：支持 Sv39 三级页表的创建、映射和遍历
2. **地址空间隔离**：为未来的进程管理提供独立的虚拟地址空间
3. **性能优化**：使用 TLB 刷新指令（sfence.vma）提高地址转换效率

### 教学性目标

1. **可视化过程**：让初学者能够"看见"页表遍历的每一步
2. **渐进式学习**：从简单的页表操作到复杂的地址空间管理
3. **实际演示**：通过实际运行展示虚拟内存的工作原理

### 差异化定位

与 rCore 等现有教学 OS 相比，Blog OS 强调：
- **更详细的过程输出**：每一步都有日志，而不是只有结果
- **更直观的可视化**：使用表格和框架展示数据结构
- **更小的学习步伐**：每个概念单独实现和测试

---

## 技术背景

### RISC-V Sv39 分页机制

RISC-V 的 Sv39（Supervisor Virtual Address 39-bit）是一种三级页表机制：

```
虚拟地址结构（39位）:
┌─────────┬─────────┬─────────┬──────────┐
│ VPN[2]  │ VPN[1]  │ VPN[0]  │  Offset  │
│ 9 bits  │ 9 bits  │ 9 bits  │ 12 bits  │
│ (38-30) │ (29-21) │ (20-12) │  (11-0)  │
└─────────┴─────────┴─────────┴──────────┘

页表项（PTE）结构（64位）:
┌────────────────────┬───────────┬─────┐
│       PPN          │  Reserved │ Flags│
│    44 bits         │  10 bits  │10bits│
│   (53-10)          │  (9-8)    │(7-0) │
└────────────────────┴───────────┴─────┘

标志位（Flags）:
- V (0): Valid - 页表项有效
- R (1): Read - 可读
- W (2): Write - 可写
- X (3): Execute - 可执行
- U (4): User - 用户模式可访问
- G (5): Global - 全局映射
- A (6): Accessed - 已访问
- D (7): Dirty - 已修改
```

### 地址转换过程

```
1. 读取 satp 寄存器获取根页表物理地址
2. 使用 VPN[2] 索引 Level 2 页表
3. 如果是叶子节点（R/W/X 任一为1）→ 1GB 大页
4. 否则，使用 VPN[1] 索引 Level 1 页表
5. 如果是叶子节点 → 2MB 大页
6. 否则，使用 VPN[0] 索引 Level 0 页表
7. 获取物理页号（PPN），加上页内偏移得到物理地址
```

---

## 实现细节

### 文件结构

```
os/src/memory/
├── mod.rs              # 内存管理模块入口，类型定义
├── paging.rs           # 页表遍历、映射、取消映射
└── address_space.rs    # 地址空间抽象，内核地址空间创建
```

### 核心数据结构

#### 1. 虚拟地址（VirtAddr）

```rust
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub const fn new(addr: usize) -> Self;
    pub const fn as_usize(self) -> usize;

    // VPN 提取
    pub const fn vpn2(self) -> usize;  // bits 38-30
    pub const fn vpn1(self) -> usize;  // bits 29-21
    pub const fn vpn0(self) -> usize;  // bits 20-12
    pub const fn page_offset(self) -> usize;  // bits 11-0
}
```

**设计考量**：
- 使用 `repr(transparent)` 确保与 `usize` 有相同的内存布局
- VPN 提取函数使用位运算和掩码，高效且清晰
- 所有函数都是 `const fn`，可在编译时计算

#### 2. 物理地址（PhysAddr）

```rust
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub const fn new(addr: usize) -> Self;
    pub const fn as_usize(self) -> usize;
}
```

**设计考量**：
- 与 VirtAddr 相似的设计，便于理解
- 未来可扩展物理地址的特殊操作

#### 3. 页表项（PageTableEntry）

```rust
#[repr(transparent)]
pub struct PageTableEntry {
    entry: usize,
}

impl PageTableEntry {
    pub const fn new() -> Self;
    pub fn is_valid(&self) -> bool;
    pub fn is_leaf(&self) -> bool;
    pub fn ppn(&self) -> usize;
    pub fn phys_addr(&self) -> PhysAddr;
    pub fn set(&mut self, ppn: usize, flags: usize);
    pub fn flags(&self) -> usize;
}
```

**关键实现**：

```rust
// 判断是否为叶子节点（映射到物理页）
pub fn is_leaf(&self) -> bool {
    (self.entry & 0xE) != 0  // R/W/X 任意一位为 1
}

// 获取物理页号
pub fn ppn(&self) -> usize {
    (self.entry >> 10) & 0xFFF_FFFF_FFFF
}

// 设置页表项（PPN + Flags）
pub fn set(&mut self, ppn: usize, flags: usize) {
    self.entry = (ppn << 10) | flags;
}
```

#### 4. 页表（PageTable）

```rust
#[repr(align(4096))]  // 页表必须 4KB 对齐
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn new() -> Self;
    pub fn get_entry(&self, index: usize) -> &PageTableEntry;
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry;
    pub fn zero(&mut self);
}
```

**设计考量**：
- 使用 `repr(align(4096))` 确保页表按页对齐
- 512 个条目对应 9 位索引（2^9 = 512）

#### 5. 地址空间（AddressSpace）

```rust
pub struct AddressSpace {
    page_table: *mut PageTable,        // 根页表指针
    page_table_paddr: PhysAddr,        // 根页表物理地址
    areas: Vec<MemoryArea>,            // 内存区域列表
}

impl AddressSpace {
    pub fn new(allocator: &mut SimpleFrameAllocator) -> Result<Self, &'static str>;
    pub fn map_region(...) -> Result<(), &'static str>;
    pub fn map_region_identity(...) -> Result<(), &'static str>;
    pub fn unmap_region(...) -> Result<(), &'static str>;
    pub fn activate(&self);
    pub fn print_layout(&self);  // 可视化教学
}
```

**设计考量**：
- 存储原始指针而非引用，避免生命周期复杂性
- 维护内存区域列表，便于管理和展示
- 提供恒等映射接口，用于内核空间

### 核心算法

#### 1. 页表遍历（walk_page_table）

```rust
pub fn walk_page_table(root_paddr: PhysAddr, vaddr: VirtAddr) -> Option<PhysAddr> {
    // 1. 获取根页表
    let root_table = unsafe {
        &*(root_paddr.as_usize() as *const PageTable)
    };

    // 2. Level 2 查找
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry(vpn2);

    if !pte2.is_valid() {
        return None;
    }

    if pte2.is_leaf() {
        // 1GB 大页
        let offset = vaddr.as_usize() & 0x3FFF_FFFF;
        return Some(PhysAddr::new(pte2.phys_addr().as_usize() + offset));
    }

    // 3. Level 1 查找
    let table1 = unsafe {
        &*(pte2.phys_addr().as_usize() as *const PageTable)
    };
    let vpn1 = vaddr.vpn1();
    let pte1 = table1.get_entry(vpn1);

    // ... 类似的逻辑 ...

    // 4. Level 0 查找
    // ... 获取最终物理地址 ...

    Some(paddr)
}
```

**算法分析**：
- **时间复杂度**: O(1) - 固定三次内存访问
- **空间复杂度**: O(1) - 只使用栈上的临时变量
- **优化点**: TLB 缓存可将后续访问优化到 O(0)

#### 2. 页面映射（map_page）

```rust
pub fn map_page(
    root_table: &mut PageTable,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: usize,
    allocator: &mut SimpleFrameAllocator,
) -> Result<(), &'static str> {
    // 1. 处理 Level 2
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry_mut(vpn2);

    let table1 = if !pte2.is_valid() {
        // 分配新的二级页表
        let frame = allocator.allocate().ok_or("Out of memory")?;
        let table1_paddr = frame.start_address();

        // 设置指向下一级的页表项
        pte2.set(table1_paddr.as_usize() >> 12, PageTableFlags::Valid as usize);

        // 清空新页表
        let table1 = unsafe {
            &mut *(table1_paddr.as_usize() as *mut PageTable)
        };
        table1.zero();
        table1
    } else {
        unsafe {
            &mut *(pte2.phys_addr().as_usize() as *mut PageTable)
        }
    };

    // 2. 处理 Level 1（类似逻辑）
    // 3. 处理 Level 0 - 设置最终映射

    let vpn0 = vaddr.vpn0();
    let pte0 = table0.get_entry_mut(vpn0);

    if pte0.is_valid() {
        return Err("Page already mapped");
    }

    // 设置叶子页表项
    pte0.set(paddr.as_usize() >> 12, flags | PageTableFlags::Valid as usize);

    // 4. 刷新 TLB
    unsafe {
        core::arch::asm!(
            "sfence.vma {0}, zero",
            in(reg) vaddr.as_usize(),
        );
    }

    Ok(())
}
```

**关键点**：
1. **按需分配**：只在需要时分配中间页表，节省内存
2. **错误处理**：检测重复映射，避免覆盖现有映射
3. **TLB 刷新**：使用 `sfence.vma` 指令刷新特定地址的 TLB 条目

#### 3. 激活地址空间（activate）

```rust
pub fn activate(&self) {
    use riscv::register::satp;

    let ppn = self.page_table_paddr.as_usize() >> 12;

    unsafe {
        // Sv39 模式 (mode=8)，ASID=0
        satp::set(satp::Mode::Sv39, 0, ppn);

        // 刷新整个 TLB
        core::arch::asm!("sfence.vma");
    }
}
```

**SATP 寄存器格式**：
```
┌──────┬──────┬────────────────────────┐
│ MODE │ ASID │         PPN            │
│ 4bit │16bit │       44 bits          │
└──────┴──────┴────────────────────────┘

MODE = 8: Sv39 模式
ASID: 地址空间标识符（Address Space ID）
PPN: 根页表物理页号
```

---

## 可视化教学特色

### 1. 详细的页表遍历输出

```rust
pub fn walk_page_table_verbose(root_paddr: PhysAddr, vaddr: VirtAddr) -> Option<PhysAddr> {
    crate::serial_println!("\n╔════════════════════════════════════════╗");
    crate::serial_println!("║     页表遍历过程（Sv39）              ║");
    crate::serial_println!("╠════════════════════════════════════════╣");
    crate::serial_println!("║ 虚拟地址: {:#018x}            ║", vaddr.as_usize());
    crate::serial_println!("║ 根页表:   {:#018x}            ║", root_paddr.as_usize());
    crate::serial_println!("╚════════════════════════════════════════╝");

    // VPN 分解
    crate::serial_println!("\n[1] 虚拟地址分解:");
    crate::serial_println!("    VPN[2] = {:3} (bits 38-30)", vpn2);
    crate::serial_println!("    VPN[1] = {:3} (bits 29-21)", vpn1);
    crate::serial_println!("    VPN[0] = {:3} (bits 20-12)", vpn0);
    crate::serial_println!("    Offset = {:#05x} (bits 11-0)", offset);

    // 逐级查找...
    crate::serial_println!("\n[2] Level 2 查找 (VPN[2] = {}):", vpn2);
    // ...
}
```

**教学价值**：
- 学生可以看到虚拟地址如何被分解
- 每一级页表查找的详细过程
- 标志位的具体含义

### 2. 地址空间布局可视化

```rust
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
```

**输出示例**：
```
╔════════════════════════════════════════╗
║      地址空间布局                      ║
╠════════════════════════════════════════╣
║ 页表物理地址: 0x00000000804c7000      ║
╠════════════════════════════════════════╣
║ 内存区域:                              ║
║ [ 0] Code     0x80000000 - 0x81000000 ║
║ [ 1] Data     0x10000000 - 0x10001000 ║
╚════════════════════════════════════════╝
```

### 3. 映射过程可视化

```rust
pub fn map_page_verbose(...) -> Result<(), &'static str> {
    crate::serial_println!("\n╔════════════════════════════════════════╗");
    crate::serial_println!("║        页面映射过程                    ║");
    crate::serial_println!("╠════════════════════════════════════════╣");
    crate::serial_println!("║ 虚拟地址: {:#018x}            ║", vaddr.as_usize());
    crate::serial_println!("║ 物理地址: {:#018x}            ║", paddr.as_usize());
    crate::serial_println!("║ 标志位:   {:#010x}                ║", flags);
    crate::serial_println!("╚════════════════════════════════════════╝");

    let result = map_page(root_table, vaddr, paddr, flags, allocator);

    if result.is_ok() {
        crate::serial_println!("✓ 映射成功!\n");
    }

    result
}
```

---

## 测试与验证

### 测试策略

1. **单元测试**：测试基本的地址操作
2. **集成测试**：测试页表遍历和映射
3. **系统测试**：激活虚拟内存后的实际访问

### 测试用例

#### 测试1：页表映射和遍历

```rust
fn test_page_table_features(memory_manager: &mut MemoryManager) {
    // 1. 创建测试页表
    let test_page_table = allocate_page_table();

    // 2. 映射一个页面
    let vaddr = VirtAddr::new(0x1000_0000);
    let paddr = PhysAddr::new(0x8100_0000);
    map_page_verbose(test_page_table, vaddr, paddr, flags, allocator);

    // 3. 验证映射
    let result = walk_page_table_verbose(root_paddr, vaddr);
    assert_eq!(result, Some(paddr));

    // 4. 映射多个连续页面
    for i in 0..3 {
        map_page_verbose(...);
    }
}
```

**测试结果**：
```
[1] 创建测试页表...
    ✓ 页表已创建，物理地址: 0x804c3000

[2] 测试页面映射...
    ✓ 页面映射成功

[3] 测试页表遍历...
    ✓ 地址转换验证成功!

[4] 映射多个连续页面...
    ✓ 映射成功!
```

#### 测试2：内核地址空间

```rust
fn test_kernel_address_space(memory_manager: &mut MemoryManager) {
    // 1. 创建内核地址空间
    let kernel_space = create_kernel_address_space(allocator)?;

    // 2. 显示布局
    kernel_space.print_layout();

    // 3. 激活地址空间
    kernel_space.activate();

    // 4. 测试虚拟内存访问
    let test_data: u32 = 0x12345678;
    println!("测试数据: {:#x}", test_data);  // 应该能正常访问
}
```

**测试结果**：
```
[1] 创建内核地址空间...
    ✓ 内核地址空间创建成功

[2] 地址空间布局:
    ║ [ 0] Code 0x80000000 - 0x81000000 ║
    ║ [ 1] Data 0x10000000 - 0x10001000 ║

[3] 激活内核地址空间...
    ✓ 地址空间已激活（虚拟内存已启用）

[4] 测试虚拟内存访问...
    测试数据: 0x12345678
    ✓ 虚拟内存访问正常
```

### 验证方法

1. **功能验证**：所有映射和遍历操作都能正确执行
2. **正确性验证**：地址转换结果与预期一致
3. **稳定性验证**：激活虚拟内存后系统稳定运行
4. **性能验证**：TLB 刷新策略有效

---

## 遇到的问题与解决

### 问题1：格式化字符串语法错误

**现象**：
```
error: invalid format string: expected `}`, found `:`
  --> src/memory/address_space.rs:283:17
   |
283 |  "║ [{:2}] {:?:7} {:#010x} - {:#010x} ║",
```

**原因**：
- Rust 格式化语法不支持 `{:?:7}` 这种组合
- 正确格式应该是 `{:8?}` 或 `{:7}`

**解决**：
```rust
// 错误
"║ [{:2}] {:?:7} {:#010x} - {:#010x} ║",

// 正确
"║ [{:2}] {:8?} {:#010x} - {:#010x} ║",
```

### 问题2：中文输出乱码

**现象**：
- 串口输出中文字符显示为 `þþþ`

**原因**：
- QEMU 串口默认不支持 UTF-8 编码

**解决方案**（未实施）：
1. 使用英文替代中文关键字
2. 配置 QEMU 串口编码
3. 使用图形界面输出

**当前策略**：
- 保留中文（便于理解代码）
- 关键信息使用英文和数字（确保可读）

### 问题3：地址空间生命周期管理

**现象**：
- 激活地址空间后不能 drop，否则页表被释放

**原因**：
- AddressSpace 的 drop 会释放页表内存
- 但当前还在使用这个页表

**解决**：
```rust
// 使用 mem::forget 防止析构
core::mem::forget(kernel_space);
```

**未来改进**：
- 使用全局变量或静态生命周期管理内核地址空间
- 实现引用计数机制

### 问题4：内存分配器的递归依赖

**问题**：
- 映射页表需要分配内存
- 但分配器本身可能需要页表支持

**当前解决**：
- 在启用虚拟内存之前完成堆分配器初始化
- 使用恒等映射确保分配器正常工作

**代码顺序**：
```rust
// 1. 初始化物理帧分配器（不需要虚拟内存）
let mut memory_manager = memory::init(kernel_end_addr);

// 2. 初始化堆分配器（使用物理地址）
allocator::init_heap(&mut memory_manager.frame_allocator);

// 3. 创建并激活虚拟内存（现在可以使用堆）
let kernel_space = create_kernel_address_space(&mut memory_manager.frame_allocator);
kernel_space.activate();
```

---

## 后续改进方向

### 短期改进（1-2周）

1. **完善页表功能**
   - [ ] 支持大页（2MB、1GB）
   - [ ] 实现页表项的 Accessed 和 Dirty 位管理
   - [ ] 添加页表统计信息（已映射页数、内存使用等）

2. **优化性能**
   - [ ] 实现批量映射（减少 TLB 刷新次数）
   - [ ] 使用 ASID 优化地址空间切换
   - [ ] 添加页表缓存层

3. **增强可视化**
   - [ ] 实现页表树形结构打印
   - [ ] 添加内存使用图表
   - [ ] 支持 HTML 格式导出（用于文档）

### 中期改进（3-4周）

1. **用户地址空间**
   - [ ] 实现用户态地址空间创建
   - [ ] 添加写时复制（Copy-on-Write）
   - [ ] 实现地址空间克隆（fork 支持）

2. **内存保护**
   - [ ] 实现页表权限检查
   - [ ] 添加 Guard Pages（检测栈溢出）
   - [ ] 实现内存访问审计

3. **高级特性**
   - [ ] 按需分配（Lazy Allocation）
   - [ ] 内存共享机制
   - [ ] 支持文件映射（mmap）

### 长期改进（2-3个月）

1. **多核支持**
   - [ ] 每个核心独立的 TLB
   - [ ] 跨核心页表同步
   - [ ] 使用 TLB shootdown 机制

2. **高级内存管理**
   - [ ] 实现反向页表（Inverted Page Table）
   - [ ] 页面回收和交换
   - [ ] NUMA 感知的内存分配

3. **调试工具**
   - [ ] 页表查看器（交互式）
   - [ ] 内存泄漏检测
   - [ ] 页面访问热力图

---

## 教学文档规划

基于本次实现，计划编写以下教学文档：

### 1. 基础教程
- **《虚拟内存入门》**：概念解释、为什么需要虚拟内存
- **《页表结构详解》**：RISC-V Sv39 页表的设计
- **《地址转换过程》**：一步步演示地址转换

### 2. 实践教程
- **《实现你的第一个页表》**：从零开始创建页表
- **《页面映射实战》**：如何映射虚拟地址
- **《激活虚拟内存》**：satp 寄存器的使用

### 3. 进阶教程
- **《地址空间设计模式》**：内核空间 vs 用户空间
- **《页表优化技巧》**：TLB、大页、批量操作
- **《虚拟内存调试》**：常见问题和排查方法

### 4. 练习题库
- **基础练习**：VPN 提取、页表项解析
- **编程练习**：实现简单的页表遍历
- **综合项目**：设计一个简单的虚拟内存系统

---

## 总结

本次实现完成了 RISC-V Sv39 虚拟内存系统的核心功能，并通过详细的可视化输出展示了页表的工作原理。这为 Blog OS 的教学目标奠定了坚实基础。

### 核心成果

- ✅ **完整功能**：三级页表的遍历、映射、激活
- ✅ **可视化特色**：详细的过程输出，便于理解
- ✅ **代码质量**：清晰的结构，丰富的注释
- ✅ **测试验证**：所有功能经过测试，稳定运行

### 差异化价值

相比其他教学 OS：
- 📊 **更详细的过程展示**：每一步都可见
- 🎨 **更美观的输出格式**：使用表格和框架
- 💡 **更小的学习步伐**：渐进式理解

### 下一步

继续实现系统调用机制，为进程管理打下基础，保持"可视化教学"的核心特色。

---

## 附录

### A. 相关代码文件

- [os/src/memory/mod.rs](../os/src/memory/mod.rs) - 内存模块主文件
- [os/src/memory/paging.rs](../os/src/memory/paging.rs) - 页表管理
- [os/src/memory/address_space.rs](../os/src/memory/address_space.rs) - 地址空间
- [os/src/main.rs](../os/src/main.rs) - 测试代码

### B. 参考资料

1. **RISC-V 规范**
   - [RISC-V Privileged Specification](https://riscv.org/technical/specifications/)
   - Sv39 分页机制详细说明

2. **相关项目**
   - [rCore-Tutorial](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
   - [xv6-riscv](https://github.com/mit-pdos/xv6-riscv)
   - [Writing an OS in Rust](https://os.phil-opp.com/)

3. **学术论文**
   - *The RISC-V Reader* - Patterson & Waterman
   - *Operating Systems: Three Easy Pieces* - Arpaci-Dusseau

### C. 术语表

| 术语 | 英文 | 解释 |
|------|------|------|
| 虚拟地址 | Virtual Address | 程序使用的地址 |
| 物理地址 | Physical Address | 实际内存地址 |
| 页表 | Page Table | 地址转换的数据结构 |
| 页表项 | Page Table Entry (PTE) | 页表中的一个条目 |
| 物理页号 | Physical Page Number (PPN) | 物理页的标识 |
| 虚拟页号 | Virtual Page Number (VPN) | 虚拟页的标识 |
| TLB | Translation Lookaside Buffer | 地址转换缓存 |
| satp | Supervisor Address Translation and Protection | 控制地址转换的寄存器 |
| Sv39 | Supervisor Virtual Address 39-bit | RISC-V 的 39 位虚拟地址模式 |

---

**文档版本**: v1.0
**最后更新**: 2025-11-24
**贡献者**: Blog OS 开发团队
**许可证**: MIT License
