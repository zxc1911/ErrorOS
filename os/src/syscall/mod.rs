/*
 * ============================================
 * 系统调用模块
 * ============================================
 * 功能：提供用户态程序与内核交互的接口
 *
 * 系统调用机制：
 * - 用户态通过 ecall 指令触发系统调用
 * - 系统调用号通过 a7 寄存器传递
 * - 参数通过 a0-a5 寄存器传递（最多6个参数）
 * - 返回值通过 a0 寄存器返回
 *
 * 支持的系统调用：
 * - sys_write: 写入数据到文件描述符
 * - sys_exit: 退出进程
 * - sys_getpid: 获取当前进程ID
 * ============================================
 */

pub mod syscall_impl;

use crate::serial_println;

/// 系统调用号定义
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SyscallId {
    Write = 64,      // sys_write
    Exit = 93,       // sys_exit
    GetPid = 172,    // sys_getpid
    Unknown = 9999,
}

impl From<usize> for SyscallId {
    fn from(id: usize) -> Self {
        match id {
            64 => SyscallId::Write,
            93 => SyscallId::Exit,
            172 => SyscallId::GetPid,
            _ => SyscallId::Unknown,
        }
    }
}

/// 系统调用上下文
///
/// 保存系统调用时的寄存器状态
#[derive(Debug, Clone, Copy)]
pub struct SyscallContext {
    /// 系统调用号 (a7)
    pub syscall_id: usize,
    /// 参数0 (a0)
    pub arg0: usize,
    /// 参数1 (a1)
    pub arg1: usize,
    /// 参数2 (a2)
    pub arg2: usize,
    /// 参数3 (a3)
    pub arg3: usize,
    /// 参数4 (a4)
    pub arg4: usize,
    /// 参数5 (a5)
    pub arg5: usize,
    /// 系统调用发生时的 PC
    pub sepc: usize,
}

impl SyscallContext {
    /// 从寄存器创建系统调用上下文
    ///
    /// # Safety
    /// 必须在系统调用异常处理时调用，此时寄存器状态有效
    pub unsafe fn from_registers() -> Self {
        let syscall_id: usize;
        let arg0: usize;
        let arg1: usize;
        let arg2: usize;
        let arg3: usize;
        let arg4: usize;
        let arg5: usize;

        core::arch::asm!(
            "mv {0}, a7",  // 读取系统调用号
            "mv {1}, a0",  // 读取参数
            "mv {2}, a1",
            "mv {3}, a2",
            "mv {4}, a3",
            "mv {5}, a4",
            "mv {6}, a5",
            out(reg) syscall_id,
            out(reg) arg0,
            out(reg) arg1,
            out(reg) arg2,
            out(reg) arg3,
            out(reg) arg4,
            out(reg) arg5,
        );

        let sepc = riscv::register::sepc::read();

        Self {
            syscall_id,
            arg0,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
            sepc,
        }
    }

    /// 设置返回值
    ///
    /// # Safety
    /// 必须在系统调用处理完成后调用
    pub unsafe fn set_return_value(&self, ret: isize) {
        core::arch::asm!(
            "mv a0, {0}",
            in(reg) ret,
        );
    }
}

/// 系统调用分发器
///
/// # 参数
/// - `context`: 系统调用上下文
///
/// # 返回
/// 系统调用返回值（通过 a0 寄存器）
pub fn syscall_dispatcher(context: &SyscallContext) -> isize {
    let syscall_id = SyscallId::from(context.syscall_id);

    // 可视化输出：显示系统调用信息
    if cfg!(feature = "verbose_syscall") {
        print_syscall_entry(context, syscall_id);
    }

    let result = match syscall_id {
        SyscallId::Write => {
            syscall_impl::sys_write(
                context.arg0,
                context.arg1 as *const u8,
                context.arg2,
            )
        }
        SyscallId::Exit => {
            syscall_impl::sys_exit(context.arg0 as i32)
        }
        SyscallId::GetPid => {
            syscall_impl::sys_getpid()
        }
        SyscallId::Unknown => {
            serial_println!(
                "[SYSCALL] Unknown syscall: {} (syscall_id={})",
                context.syscall_id,
                context.syscall_id
            );
            -1 // ENOSYS
        }
    };

    // 可视化输出：显示返回结果
    if cfg!(feature = "verbose_syscall") {
        print_syscall_exit(syscall_id, result);
    }

    result
}

/// 打印系统调用入口信息（可视化）
fn print_syscall_entry(context: &SyscallContext, syscall_id: SyscallId) {
    serial_println!("\n╔════════════════════════════════════════╗");
    serial_println!("║     系统调用追踪                       ║");
    serial_println!("╠════════════════════════════════════════╣");
    serial_println!("║ 调用号: {:?} ({})", syscall_id, context.syscall_id);
    serial_println!("║ PC: {:#x}", context.sepc);
    serial_println!("╠════════════════════════════════════════╣");
    serial_println!("║ 参数:                                  ║");
    serial_println!("║   a0 (arg0) = {:#x}", context.arg0);
    serial_println!("║   a1 (arg1) = {:#x}", context.arg1);
    serial_println!("║   a2 (arg2) = {:#x}", context.arg2);
    serial_println!("╠════════════════════════════════════════╣");
}

/// 打印系统调用返回信息（可视化）
fn print_syscall_exit(syscall_id: SyscallId, result: isize) {
    serial_println!("║ 返回值: {} ({:#x})", result, result as usize);
    serial_println!("╚════════════════════════════════════════╝\n");
}

// ============================================
// 用于测试的简化版系统调用函数
// ============================================

/// 简化的系统调用接口（用于内核测试）
pub fn test_syscall(syscall_id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let context = SyscallContext {
        syscall_id,
        arg0,
        arg1,
        arg2,
        arg3: 0,
        arg4: 0,
        arg5: 0,
        sepc: 0,
    };
    syscall_dispatcher(&context)
}
