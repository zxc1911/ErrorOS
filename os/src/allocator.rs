/*
 * ============================================
 * RISC-V 堆分配器模块
 * ============================================
 * 功能：提供内核堆内存分配
 * 实现：使用固定大小块分配器
 *
 * 堆配置：
 * - 起始地址：0x8040_0000（物理内存中的某个位置）
 * - 大小：1 MB
 * ============================================
 */

// ============================================
// 堆配置
// ============================================

/// 堆起始地址（RISC-V 物理内存空间）
pub const HEAP_START: usize = 0x8040_0000;

/// 堆大小（1 MB）
pub const HEAP_SIZE: usize = 1024 * 1024;

// ============================================
// 分配器实现
// ============================================

pub mod bump;
pub mod linked_list;
pub mod fixed_size_block;

use fixed_size_block::FixedSizeBlockAllocator;

/// 互斥锁包装器
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// 全局分配器实例
#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> =
    Locked::new(FixedSizeBlockAllocator::new());

/// 对齐地址到指定边界
///
/// # 参数
/// - `addr`: 要对齐的地址
/// - `align`: 对齐边界（必须是 2 的幂）
///
/// # 返回
/// 对齐后的地址
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// 初始化堆分配器（简单版本，不需要虚拟内存）
///
/// # 功能
/// - 直接在物理内存中初始化堆
/// - 不需要页表或虚拟内存支持
///
/// # 参数
/// - `kernel_end_addr`: 内核结束地址
pub fn init_heap_simple(
    kernel_end_addr: usize,
) -> Result<(), &'static str> {
    use crate::serial_println;

    // 将堆起始地址设置为内核结束地址之后，对齐到 4KB
    let heap_start = align_up(kernel_end_addr, 4096);

    serial_println!("[ALLOCATOR] Initializing heap at {:#x}", heap_start);
    serial_println!("[ALLOCATOR] Heap size: {} bytes", HEAP_SIZE);

    // 初始化分配器
    unsafe {
        ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
    }

    serial_println!("[ALLOCATOR] Heap initialized successfully");
    Ok(())
}

/// 初始化堆分配器（完整版本，需要虚拟内存）
///
/// # 功能
/// - 为堆区域分配物理帧
/// - 映射堆区域到虚拟地址空间
/// - 初始化全局分配器
///
/// # 参数
/// - `frame_allocator`: 物理帧分配器
///
/// # 注意
/// 此函数需要虚拟内存模块支持，当前已禁用
#[allow(dead_code)]
pub fn init_heap(
    #[allow(unused_variables)] frame_allocator: &mut (),
) -> Result<(), &'static str> {
    Err("Virtual memory not implemented")
}

/*
// 原始的 init_heap 实现（需要虚拟内存）
pub fn init_heap(
    frame_allocator: &mut crate::memory::SimpleFrameAllocator,
) -> Result<(), &'static str> {
    use crate::{serial_println, memory::PAGE_SIZE};

    serial_println!("[ALLOCATOR] Initializing heap at {:#x}", HEAP_START);
    serial_println!("[ALLOCATOR] Heap size: {} bytes", HEAP_SIZE);

    // 计算需要的页数
    let page_count = (HEAP_SIZE + PAGE_SIZE - 1) / PAGE_SIZE;
    serial_println!("[ALLOCATOR] Allocating {} pages for heap", page_count);

    // 分配物理帧
    for _i in 0..page_count {
        let _frame = frame_allocator
            .allocate()
            .ok_or("Failed to allocate frame for heap")?;

        // 注释掉详细的分配输出以避免中断期间的竞态条件
        // serial_println!(
        //     "[ALLOCATOR] Allocated frame {} at {:#x}",
        //     i,
        //     frame.start_address().as_usize()
        // );
    }

    // 初始化分配器
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    serial_println!("[ALLOCATOR] Heap initialized successfully");
    Ok(())
}
*/

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{boxed::Box, vec::Vec};

    #[test_case]
    fn test_heap_allocation() {
        let heap_value = Box::new(41);
        assert_eq!(*heap_value, 41);
    }

    #[test_case]
    fn test_large_vec() {
        let n = 1000;
        let mut vec = Vec::new();
        for i in 0..n {
            vec.push(i);
        }
        assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
    }

    #[test_case]
    fn test_many_boxes() {
        for i in 0..10000 {
            let x = Box::new(i);
            assert_eq!(*x, i);
        }
    }
}
