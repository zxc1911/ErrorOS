#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::arch::global_asm;
use core::panic::PanicInfo;
use os::println;
use os::task::keyboard;
use os::task::executor::Executor;

/// RISC-V 汇编入口点
///
/// 定义在汇编中，负责：
/// - 清零 BSS 段
/// - 设置栈指针
/// - 跳转到 kernel_main
global_asm!(
    ".section .text.entry",
    ".globl _start",
    "_start:",
    // 设置栈指针
    "   la sp, stack_end",
    // 清零 BSS 段
    "   la t0, bss_start",
    "   la t1, bss_end",
    "1:",
    "   bgeu t0, t1, 2f",
    "   sd zero, (t0)",
    "   addi t0, t0, 8",
    "   j 1b",
    "2:",
    // 跳转到 kernel_main
    "   call kernel_main",
    // 如果返回，进入死循环
    "3:",
    "   wfi",
    "   j 3b",
);
/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os::hlt_loop();            // new
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}
extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
use os::task::Task;

/// 内核主函数
///
/// # 功能
/// - 初始化内核
/// - 设置内存管理
/// - 启动异步执行器
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    use os::memory;
    use os::allocator;

    println!("Welcome to Error OS{}", "!");
    os::init();

    // 获取内核结束地址（由链接器定义）
    extern "C" {
        static kernel_end: u8;
    }
    let kernel_end_addr = unsafe { &kernel_end as *const u8 as usize };

    // 初始化内存管理
    let mut memory_manager = memory::init(kernel_end_addr);

    allocator::init_heap(&mut memory_manager.frame_allocator)
        .expect("heap initialization failed");

    let heap_value=Box::new(41);
    println!("heap_value {:p}",heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}",Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    // ========================================
    // 测试页表功能（可视化教学演示）
    // ========================================
    println!("\n========================================");
    println!("  页表功能测试（教学演示）");
    println!("========================================\n");
    test_page_table_features(&mut memory_manager);

    // ========================================
    // 创建并激活内核地址空间
    // ========================================
    println!("\n========================================");
    println!("  创建内核地址空间");
    println!("========================================\n");
    test_kernel_address_space(&mut memory_manager);

    // 启动异步执行器
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    println!("It did not crash!");
    os::hlt_loop();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

/// 测试页表功能（教学演示）
///
/// # 功能
/// - 创建测试页表
/// - 演示页面映射过程
/// - 演示页表遍历过程
/// - 展示可视化教学效果
fn test_page_table_features(memory_manager: &mut os::memory::MemoryManager) {
    use os::memory::{VirtAddr, PhysAddr, PageTable, PageTableFlags};
    use os::memory::{map_page_verbose, walk_page_table_verbose};

    println!("[1] 创建测试页表...");

    // 分配一个页表
    let test_page_table_frame = memory_manager.frame_allocator
        .allocate()
        .expect("Failed to allocate page table");

    let test_page_table = unsafe {
        &mut *(test_page_table_frame.start_address().as_usize() as *mut PageTable)
    };
    test_page_table.zero();

    println!("    ✓ 页表已创建，物理地址: {:#x}\n",
        test_page_table_frame.start_address().as_usize());

    // 测试1：映射一个页面
    println!("[2] 测试页面映射...");
    let vaddr = VirtAddr::new(0x1000_0000);  // 虚拟地址
    let paddr = PhysAddr::new(0x8100_0000);  // 物理地址

    let flags = PageTableFlags::Read as usize
        | PageTableFlags::Write as usize
        | PageTableFlags::Valid as usize;

    // 使用可视化版本的 map_page
    match map_page_verbose(
        test_page_table,
        vaddr,
        paddr,
        flags,
        &mut memory_manager.frame_allocator
    ) {
        Ok(_) => println!("    ✓ 页面映射成功\n"),
        Err(e) => println!("    ✗ 页面映射失败: {}\n", e),
    }

    // 测试2：遍历页表验证映射
    println!("[3] 测试页表遍历...");
    let root_paddr = test_page_table_frame.start_address();

    // 使用可视化版本的 walk_page_table
    match walk_page_table_verbose(root_paddr, vaddr) {
        Some(result_paddr) => {
            if result_paddr.as_usize() == paddr.as_usize() {
                println!("    ✓ 地址转换验证成功!\n");
            } else {
                println!("    ✗ 地址转换错误: 期望 {:#x}, 得到 {:#x}\n",
                    paddr.as_usize(), result_paddr.as_usize());
            }
        }
        None => println!("    ✗ 页面未映射\n"),
    }

    // 测试3：映射多个页面
    println!("[4] 映射多个连续页面...");
    for i in 0..3 {
        let v = VirtAddr::new(0x2000_0000 + i * 0x1000);
        let p = PhysAddr::new(0x8200_0000 + i * 0x1000);

        match map_page_verbose(
            test_page_table,
            v,
            p,
            flags,
            &mut memory_manager.frame_allocator
        ) {
            Ok(_) => {},
            Err(e) => println!("    ✗ 页面 {} 映射失败: {}", i, e),
        }
    }

    println!("\n========================================");
    println!("  页表功能测试完成！");
    println!("========================================\n");
}

/// 测试内核地址空间（教学演示）
///
/// # 功能
/// - 创建内核地址空间
/// - 映射内核代码段和设备
/// - 可视化显示地址空间布局
/// - 激活地址空间
fn test_kernel_address_space(memory_manager: &mut os::memory::MemoryManager) {
    use os::memory::create_kernel_address_space;

    println!("[1] 创建内核地址空间...");

    // 创建内核地址空间
    let kernel_space = match create_kernel_address_space(&mut memory_manager.frame_allocator) {
        Ok(space) => {
            println!("    ✓ 内核地址空间创建成功\n");
            space
        }
        Err(e) => {
            println!("    ✗ 创建失败: {}\n", e);
            return;
        }
    };

    // 显示地址空间布局（可视化教学）
    println!("[2] 地址空间布局:");
    kernel_space.print_layout();

    // 激活地址空间
    println!("[3] 激活内核地址空间...");
    kernel_space.activate();
    println!("    ✓ 地址空间已激活（虚拟内存已启用）\n");

    // 测试：在虚拟内存下访问内核数据
    println!("[4] 测试虚拟内存访问...");
    let test_data: u32 = 0x12345678;
    println!("    测试数据: {:#x}", test_data);
    println!("    数据地址: {:p}", &test_data);
    println!("    ✓ 虚拟内存访问正常\n");

    println!("========================================");
    println!("  虚拟内存系统已启用！");
    println!("========================================\n");

    // 重要：这里不能drop kernel_space，因为我们还在使用它
    // 所以使用 core::mem::forget 防止析构
    core::mem::forget(kernel_space);
}