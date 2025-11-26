/*
 * ============================================
 * RISC-V 操作系统库入口
 * ============================================
 * 功能：提供操作系统的核心功能库
 * 架构：RISC-V 64
 *
 * 主要模块：
 * - 串口输出（serial）
 * - 控制台（console）
 * - 中断处理（interrupts）
 * - 内存管理（memory）
 * - 堆分配器（allocator）
 * - 异步任务（task）
 * ============================================
 */

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_riscv_interrupt)]  // RISC-V 中断 ABI（实验性功能）

use core::panic::PanicInfo;

// ============================================
// 模块声明
// ============================================

pub mod serial;      // 串口驱动
pub mod console;     // 控制台输出
pub mod interrupts;  // 中断和异常处理
pub mod allocator;   // 堆分配器
pub mod task;        // 异步任务系统

// ============================================
// 外部 crate
// ============================================

extern crate alloc;  // 启用 alloc crate（堆分配）

// ============================================
// 测试框架
// ============================================

/// 测试特征
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// 测试运行器
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// 测试 panic 处理
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

// ============================================
// QEMU 退出码
// ============================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// 退出 QEMU
///
/// # 说明
/// 在 RISC-V QEMU 中，我们使用 SBI 的 shutdown 调用
pub fn exit_qemu(exit_code: QemuExitCode) {
    // RISC-V SBI shutdown
    // 注意：在实际的 QEMU 环境中，需要 SBI 支持
    // 这里我们使用一个简单的实现
    serial_println!("[QEMU] Exiting with code {:?}", exit_code);

    // 触发 shutdown（通过 SBI 调用）
    // ecall with a7=8 (SBI shutdown)
    unsafe {
        core::arch::asm!(
            "li a7, 8",      // SBI shutdown 扩展
            "li a6, 0",      // function ID 0
            "li a0, 0",      // type = 0 (shutdown)
            "li a1, 0",      // reason = 0
            "ecall",
            options(noreturn)
        );
    }
}

// ============================================
// 初始化函数
// ============================================

/// 初始化操作系统
///
/// # 功能
/// - 初始化中断描述符表
/// - 启用中断
pub fn init() {
    serial_println!("[INIT] Initializing RISC-V OS");

    // 初始化中断系统
    interrupts::init_idt();

    // 启用中断
    interrupts::enable_interrupts();

    serial_println!("[INIT] Initialization complete");
}

/// 无限循环（使用 wfi 指令节能）
///
/// # 说明
/// RISC-V 的 wfi (Wait For Interrupt) 指令
/// 让 CPU 进入低功耗状态，直到中断到来
pub fn hlt_loop() -> ! {
    loop {
        riscv::asm::wfi();
    }
}

// ============================================
// Panic 处理（仅测试模式）
// ============================================

/// Panic 处理器（测试模式）
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

// ============================================
// 测试入口点
// ============================================

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    hlt_loop();
}
