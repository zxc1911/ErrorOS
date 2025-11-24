/*
 * ============================================
 * 系统调用具体实现
 * ============================================
 */

use crate::serial_println;

/// sys_write - 写入数据到文件描述符
///
/// # 参数
/// - `fd`: 文件描述符 (1=stdout, 2=stderr)
/// - `buf`: 数据缓冲区指针
/// - `len`: 数据长度
///
/// # 返回
/// 成功写入的字节数，或错误码（负数）
///
/// # 教学说明
/// 这是最基础的系统调用之一，用于输出数据。
/// 在完整的OS中，需要：
/// 1. 验证文件描述符有效性
/// 2. 检查缓冲区指针合法性（在用户空间范围内）
/// 3. 根据文件描述符类型调用相应的写入函数
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    // 参数验证
    if buf.is_null() {
        serial_println!("[SYSCALL] sys_write: invalid buffer pointer");
        return -1; // EFAULT
    }

    // 目前只支持 stdout (1) 和 stderr (2)
    match fd {
        1 | 2 => {
            // 将用户空间的缓冲区转换为字符串
            let slice = unsafe {
                core::slice::from_raw_parts(buf, len)
            };

            // 尝试转换为 UTF-8 字符串
            match core::str::from_utf8(slice) {
                Ok(s) => {
                    // 使用串口输出
                    crate::serial_print!("{}", s);
                    len as isize
                }
                Err(_) => {
                    // 非 UTF-8 数据，按字节输出
                    for &byte in slice {
                        crate::serial_print!("{}", byte as char);
                    }
                    len as isize
                }
            }
        }
        _ => {
            serial_println!("[SYSCALL] sys_write: unsupported fd={}", fd);
            -1 // EBADF (Bad file descriptor)
        }
    }
}

/// sys_exit - 退出当前进程
///
/// # 参数
/// - `exit_code`: 退出码
///
/// # 返回
/// 此函数不返回
///
/// # 教学说明
/// 退出系统调用会：
/// 1. 清理进程资源（页表、内存、文件描述符等）
/// 2. 将进程状态设置为 Zombie
/// 3. 通知父进程
/// 4. 调度到其他进程
///
/// 目前简化实现：直接进入死循环
pub fn sys_exit(exit_code: i32) -> isize {
    serial_println!("\n╔════════════════════════════════════════╗");
    serial_println!("║     进程退出                           ║");
    serial_println!("╠════════════════════════════════════════╣");
    serial_println!("║ 退出码: {}", exit_code);
    serial_println!("╚════════════════════════════════════════╝\n");

    // TODO: 在实现进程管理后，这里应该：
    // 1. 回收进程资源
    // 2. 切换到调度器
    // 3. 选择下一个进程运行

    // 目前简化实现：进入 hlt_loop
    crate::hlt_loop();
}

/// sys_getpid - 获取当前进程ID
///
/// # 返回
/// 当前进程的 PID
///
/// # 教学说明
/// 这是一个简单的只读系统调用。
/// 在完整的OS中，需要：
/// 1. 从当前 CPU 的运行队列获取当前进程
/// 2. 返回进程控制块中的 PID 字段
///
/// 目前简化实现：返回固定值 1（假设只有 init 进程）
pub fn sys_getpid() -> isize {
    // TODO: 在实现进程管理后，返回真实的 PID
    // 目前返回固定值 1
    1
}

// ============================================
// 系统调用辅助函数
// ============================================

/// 验证用户空间指针是否有效
///
/// # 参数
/// - `ptr`: 要验证的指针
/// - `len`: 内存区域长度
///
/// # 返回
/// true 表示有效，false 表示无效
///
/// # 教学说明
/// 在真实OS中，需要检查：
/// 1. 指针是否在用户空间范围内
/// 2. 对应的页表项是否存在
/// 3. 是否有相应的访问权限
///
/// 这是防止用户程序访问内核内存的重要安全机制
#[allow(dead_code)]
fn validate_user_pointer(ptr: *const u8, len: usize) -> bool {
    // TODO: 实现真实的指针验证逻辑
    // 目前简化实现：只检查是否为空
    !ptr.is_null() && len > 0
}

/// 从用户空间复制字符串到内核空间
///
/// # 参数
/// - `user_ptr`: 用户空间字符串指针
/// - `max_len`: 最大长度
///
/// # 返回
/// 复制的字符串，或 None
///
/// # 教学说明
/// 在用户态和内核态之间传递数据时，需要：
/// 1. 验证用户空间地址有效性
/// 2. 复制数据到内核空间（避免用户修改）
/// 3. 处理可能的页错误
#[allow(dead_code)]
fn copy_string_from_user(user_ptr: *const u8, max_len: usize) -> Option<alloc::string::String> {
    use alloc::string::String;

    if user_ptr.is_null() {
        return None;
    }

    // 查找字符串结尾（'\0'）
    let mut len = 0;
    while len < max_len {
        unsafe {
            let byte = user_ptr.add(len).read();
            if byte == 0 {
                break;
            }
            len += 1;
        }
    }

    // 复制字符串
    let slice = unsafe { core::slice::from_raw_parts(user_ptr, len) };
    String::from_utf8(slice.to_vec()).ok()
}
