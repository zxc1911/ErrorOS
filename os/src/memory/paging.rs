/*
 * ============================================
 * RISC-V 页表管理模块
 * ============================================
 * 功能：页表遍历、映射、取消映射
 *
 * 教学特色：
 * - 详细的过程日志输出
 * - 每一步都可视化
 * - 便于理解 Sv39 三级页表机制
 * ============================================
 */

use super::{PhysAddr, VirtAddr, PageTable, PageTableEntry, PageTableFlags, PhysFrame, SimpleFrameAllocator, PAGE_SIZE};

/// 遍历页表，将虚拟地址转换为物理地址
///
/// # 参数
/// - `root_paddr`: 根页表的物理地址
/// - `vaddr`: 要转换的虚拟地址
///
/// # 返回
/// - Some(PhysAddr): 转换成功
/// - None: 页面未映射
///
/// # 教学说明
/// RISC-V Sv39 使用三级页表：
/// - Level 2: VPN[2] (bits 38-30)
/// - Level 1: VPN[1] (bits 29-21)
/// - Level 0: VPN[0] (bits 20-12)
/// - Offset: bits 11-0
pub fn walk_page_table(root_paddr: PhysAddr, vaddr: VirtAddr) -> Option<PhysAddr> {
    // 获取根页表指针
    let root_table = unsafe {
        &*(root_paddr.as_usize() as *const PageTable)
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
        &*(pte2.phys_addr().as_usize() as *const PageTable)
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
        &*(pte1.phys_addr().as_usize() as *const PageTable)
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

/// 可视化页表遍历（教学版本，带详细输出）
///
/// # 教学特色
/// 这个函数会打印每一步的查找过程，帮助理解页表遍历
pub fn walk_page_table_verbose(root_paddr: PhysAddr, vaddr: VirtAddr) -> Option<PhysAddr> {
    crate::serial_println!("\n╔════════════════════════════════════════╗");
    crate::serial_println!("║     页表遍历过程（Sv39）              ║");
    crate::serial_println!("╠════════════════════════════════════════╣");
    crate::serial_println!("║ 虚拟地址: {:#018x}            ║", vaddr.as_usize());
    crate::serial_println!("║ 根页表:   {:#018x}            ║", root_paddr.as_usize());
    crate::serial_println!("╚════════════════════════════════════════╝");

    // VPN 分解
    let vpn2 = vaddr.vpn2();
    let vpn1 = vaddr.vpn1();
    let vpn0 = vaddr.vpn0();
    let offset = vaddr.page_offset();

    crate::serial_println!("\n[1] 虚拟地址分解:");
    crate::serial_println!("    VPN[2] = {:3} (bits 38-30)", vpn2);
    crate::serial_println!("    VPN[1] = {:3} (bits 29-21)", vpn1);
    crate::serial_println!("    VPN[0] = {:3} (bits 20-12)", vpn0);
    crate::serial_println!("    Offset = {:#05x} (bits 11-0)", offset);

    let root_table = unsafe {
        &*(root_paddr.as_usize() as *const PageTable)
    };

    // Level 2
    crate::serial_println!("\n[2] Level 2 查找 (VPN[2] = {}):", vpn2);
    let pte2 = root_table.get_entry(vpn2);

    if !pte2.is_valid() {
        crate::serial_println!("    ✗ 页表项无效 (V=0)");
        return None;
    }

    crate::serial_println!("    ✓ 页表项有效");
    crate::serial_println!("    PPN = {:#x}", pte2.ppn());
    crate::serial_println!("    Flags = {:#04x}", pte2.flags());

    if pte2.is_leaf() {
        crate::serial_println!("    → 这是一个 1GB 大页");
        let paddr = PhysAddr::new(pte2.phys_addr().as_usize() + (vaddr.as_usize() & 0x3FFF_FFFF));
        crate::serial_println!("\n✓ 转换完成: {:#x} → {:#x}\n", vaddr.as_usize(), paddr.as_usize());
        return Some(paddr);
    }

    crate::serial_println!("    → 指向下一级页表");

    // Level 1
    let table1_paddr = pte2.phys_addr();
    crate::serial_println!("\n[3] Level 1 查找 (VPN[1] = {}):", vpn1);
    crate::serial_println!("    页表地址: {:#x}", table1_paddr.as_usize());

    let table1 = unsafe {
        &*(table1_paddr.as_usize() as *const PageTable)
    };
    let pte1 = table1.get_entry(vpn1);

    if !pte1.is_valid() {
        crate::serial_println!("    ✗ 页表项无效 (V=0)");
        return None;
    }

    crate::serial_println!("    ✓ 页表项有效");
    crate::serial_println!("    PPN = {:#x}", pte1.ppn());
    crate::serial_println!("    Flags = {:#04x}", pte1.flags());

    if pte1.is_leaf() {
        crate::serial_println!("    → 这是一个 2MB 大页");
        let paddr = PhysAddr::new(pte1.phys_addr().as_usize() + (vaddr.as_usize() & 0x1F_FFFF));
        crate::serial_println!("\n✓ 转换完成: {:#x} → {:#x}\n", vaddr.as_usize(), paddr.as_usize());
        return Some(paddr);
    }

    crate::serial_println!("    → 指向下一级页表");

    // Level 0
    let table0_paddr = pte1.phys_addr();
    crate::serial_println!("\n[4] Level 0 查找 (VPN[0] = {}):", vpn0);
    crate::serial_println!("    页表地址: {:#x}", table0_paddr.as_usize());

    let table0 = unsafe {
        &*(table0_paddr.as_usize() as *const PageTable)
    };
    let pte0 = table0.get_entry(vpn0);

    if !pte0.is_valid() {
        crate::serial_println!("    ✗ 页表项无效 (V=0)");
        return None;
    }

    crate::serial_println!("    ✓ 页表项有效 (4KB 页)");
    crate::serial_println!("    PPN = {:#x}", pte0.ppn());
    crate::serial_println!("    Flags = {:#04x}", pte0.flags());

    let paddr = PhysAddr::new(pte0.phys_addr().as_usize() + offset);
    crate::serial_println!("\n[5] 最终物理地址:");
    crate::serial_println!("    物理页帧: {:#x}", pte0.phys_addr().as_usize());
    crate::serial_println!("    页内偏移: {:#x}", offset);
    crate::serial_println!("    物理地址: {:#x}", paddr.as_usize());

    crate::serial_println!("\n✓ 转换完成: {:#x} → {:#x}\n", vaddr.as_usize(), paddr.as_usize());

    Some(paddr)
}

/// 映射单个页面
///
/// # 参数
/// - `root_table`: 根页表（可变引用）
/// - `vaddr`: 虚拟地址
/// - `paddr`: 物理地址
/// - `flags`: 页表标志位
/// - `allocator`: 帧分配器（用于分配中间页表）
///
/// # 教学说明
/// 这个函数会自动分配中间页表（如果需要）
pub fn map_page(
    root_table: &mut PageTable,
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
            &mut *(table1_paddr.as_usize() as *mut PageTable)
        };
        table1.zero();
        table1
    } else {
        unsafe {
            &mut *(pte2.phys_addr().as_usize() as *mut PageTable)
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
            &mut *(table0_paddr.as_usize() as *mut PageTable)
        };
        table0.zero();
        table0
    } else {
        unsafe {
            &mut *(pte1.phys_addr().as_usize() as *mut PageTable)
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
        // RISC-V sfence.vma 指令
        core::arch::asm!(
            "sfence.vma {0}, zero",
            in(reg) vaddr.as_usize(),
        );
    }

    Ok(())
}

/// 可视化页面映射（教学版本）
pub fn map_page_verbose(
    root_table: &mut PageTable,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: usize,
    allocator: &mut SimpleFrameAllocator,
) -> Result<(), &'static str> {
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
    } else {
        crate::serial_println!("✗ 映射失败: {:?}\n", result);
    }

    result
}

/// 取消页面映射
///
/// # 返回
/// - Ok(PhysAddr): 原来映射的物理地址
/// - Err: 页面未映射
pub fn unmap_page(
    root_table: &mut PageTable,
    vaddr: VirtAddr,
) -> Result<PhysAddr, &'static str> {
    // 遍历页表找到最后一级
    let vpn2 = vaddr.vpn2();
    let pte2 = root_table.get_entry_mut(vpn2);

    if !pte2.is_valid() {
        return Err("Page not mapped");
    }

    let table1 = unsafe {
        &mut *(pte2.phys_addr().as_usize() as *mut PageTable)
    };

    let vpn1 = vaddr.vpn1();
    let pte1 = table1.get_entry_mut(vpn1);

    if !pte1.is_valid() {
        return Err("Page not mapped");
    }

    let table0 = unsafe {
        &mut *(pte1.phys_addr().as_usize() as *mut PageTable)
    };

    let vpn0 = vaddr.vpn0();
    let pte0 = table0.get_entry_mut(vpn0);

    if !pte0.is_valid() {
        return Err("Page not mapped");
    }

    let paddr = pte0.phys_addr();

    // 清除页表项
    *pte0 = PageTableEntry::new();

    // 刷新 TLB
    unsafe {
        core::arch::asm!(
            "sfence.vma {0}, zero",
            in(reg) vaddr.as_usize(),
        );
    }

    Ok(paddr)
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
