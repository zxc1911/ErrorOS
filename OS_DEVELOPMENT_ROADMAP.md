# RISC-V 操作系统开发路线图

## 目录
1. [当前进度评估](#当前进度评估)
2. [开发阶段规划](#开发阶段规划)
3. [详细实现步骤](#详细实现步骤)
4. [系统调用实现路径](#系统调用实现路径)

---

## 当前进度评估

### ✅ 已完成的核心功能

| 模块 | 功能 | 完成度 | 说明 |
|------|------|--------|------|
| **启动引导** | 内核入口点 | ✅ 100% | RISC-V 汇编入口，BSS 清零，栈设置 |
| **串口驱动** | UART 输出 | ✅ 100% | 支持格式化输出，调试信息 |
| **控制台** | VGA 文本显示 | ✅ 100% | 彩色输出，滚动支持 |
| **中断系统** | 异常/中断处理 | ✅ 90% | 时钟中断、异常处理基础框架 |
| **内存管理** | 物理帧分配 | ⚠️ 60% | 简单的顺序分配器，缺少回收 |
| **堆分配器** | 动态内存分配 | ✅ 80% | Bump/链表/固定大小块分配器 |
| **异步任务** | 协作式调度 | ✅ 70% | 异步执行器，键盘任务 |

### ❌ 缺失的关键功能

| 模块 | 优先级 | 说明 |
|------|--------|------|
| **虚拟内存** | 🔴 高 | 页表管理，地址空间隔离 |
| **进程管理** | 🔴 高 | 进程创建、调度、上下文切换 |
| **系统调用** | 🔴 高 | 用户态/内核态切换 |
| **文件系统** | 🟡 中 | VFS 抽象层，具体 FS 实现 |
| **设备驱动** | 🟡 中 | 块设备、字符设备框架 |
| **用户程序** | 🟡 中 | ELF 加载器，用户态执行 |
| **网络栈** | 🟢 低 | TCP/IP 协议栈 |
| **多核支持** | 🟢 低 | SMP 调度，核间通信 |

---

## 开发阶段规划

### 第一阶段：完善内存管理（2-3周）
**目标**：建立完整的虚拟内存系统，为进程隔离打下基础

#### 1.1 完善物理内存管理
- [ ] 实现帧回收机制（释放物理页）
- [ ] 添加引用计数（支持共享内存）
- [ ] 实现位图分配器（提高分配效率）
- [ ] 内存统计和监控功能

#### 1.2 实现虚拟内存管理
- [ ] 页表遍历和地址转换
- [ ] 页表项的创建和映射
- [ ] 页面权限管理（R/W/X/U）
- [ ] 按需分配（lazy allocation）
- [ ] 写时复制（Copy-on-Write）

#### 1.3 设置内核地址空间
- [ ] 恒等映射（内核直接访问物理内存）
- [ ] 高半核（Higher Half Kernel）布局
- [ ] 内核堆的虚拟内存映射
- [ ] Guard Pages（防止栈溢出）

**关键文件**：
- `os/src/memory/paging.rs` - 页表管理
- `os/src/memory/frame_allocator.rs` - 物理帧分配器重构
- `os/src/memory/mapper.rs` - 虚拟地址映射

**检验标准**：
```rust
// 能够创建独立的地址空间
let addr_space = AddressSpace::new();
// 能够映射虚拟地址到物理地址
addr_space.map(virt_addr, phys_addr, flags);
// 能够切换地址空间
addr_space.activate();
```

---

### 第二阶段：实现系统调用接口（1-2周）
**目标**：建立用户态和内核态的通信桥梁

#### 2.1 系统调用基础设施
- [ ] ecall 指令处理（UserEnvCall 异常）
- [ ] 系统调用号定义（syscall numbers）
- [ ] 参数传递规范（a0-a5 寄存器）
- [ ] 返回值处理

#### 2.2 基础系统调用实现
- [ ] `sys_write` - 输出到控制台
- [ ] `sys_read` - 从控制台读取
- [ ] `sys_exit` - 进程退出
- [ ] `sys_yield` - 主动让出 CPU
- [ ] `sys_getpid` - 获取进程 ID
- [ ] `sys_sleep` - 睡眠指定时间

#### 2.3 系统调用封装
- [ ] 用户态系统调用包装函数
- [ ] 错误码定义（POSIX 兼容）
- [ ] 系统调用测试程序

**关键文件**：
- `os/src/syscall/mod.rs` - 系统调用分发
- `os/src/syscall/process.rs` - 进程相关系统调用
- `os/src/syscall/fs.rs` - 文件系统系统调用
- `user/src/syscall.rs` - 用户态封装

**实现示例**：
```rust
// 内核态
pub fn syscall_handler(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_EXIT => sys_exit(args[0] as i32),
        _ => -1, // ENOSYS
    }
}

// 用户态
pub fn write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYS_WRITE, [fd, buf.as_ptr() as usize, buf.len(), 0, 0, 0])
}
```

---

### 第三阶段：进程管理（3-4周）
**目标**：实现完整的进程抽象和多任务调度

#### 3.1 进程控制块（PCB）设计
- [ ] 进程状态（Ready/Running/Blocked/Zombie）
- [ ] 上下文保存（寄存器、栈指针、PC）
- [ ] 地址空间指针
- [ ] 父子进程关系
- [ ] 文件描述符表
- [ ] 工作目录

#### 3.2 进程创建和销毁
- [ ] `fork()` - 创建子进程
- [ ] `exec()` - 加载新程序
- [ ] `wait()` - 等待子进程
- [ ] `exit()` - 进程退出
- [ ] 孤儿进程和僵尸进程处理

#### 3.3 上下文切换
- [ ] 保存/恢复所有通用寄存器
- [ ] 保存/恢复特殊寄存器（sstatus, sepc）
- [ ] 切换页表（satp 寄存器）
- [ ] 切换用户栈和内核栈
- [ ] TLS（线程本地存储）支持

#### 3.4 进程调度器
- [ ] 时间片轮转（Round-Robin）
- [ ] 优先级调度
- [ ] 实时调度（FIFO/RR）
- [ ] 多级反馈队列（MLFQ）
- [ ] 调度统计信息

#### 3.5 线程支持
- [ ] 内核线程实现
- [ ] 用户线程支持
- [ ] 线程同步原语（Mutex, Semaphore, CondVar）
- [ ] 线程本地存储（TLS）

**关键文件**：
- `os/src/process/mod.rs` - 进程管理核心
- `os/src/process/pcb.rs` - 进程控制块
- `os/src/process/scheduler.rs` - 调度器
- `os/src/process/context.rs` - 上下文切换
- `os/src/process/thread.rs` - 线程实现

**PCB 数据结构**：
```rust
pub struct Process {
    pub pid: Pid,
    pub parent: Option<Pid>,
    pub state: ProcessState,
    pub context: Context,
    pub address_space: AddressSpace,
    pub file_table: FileTable,
    pub working_dir: PathBuf,
    pub children: Vec<Pid>,
    pub exit_code: Option<i32>,
}
```

---

### 第四阶段：文件系统抽象层（2-3周）
**目标**：建立 VFS 框架，支持基本文件操作

#### 4.1 VFS（虚拟文件系统）设计
- [ ] Inode 抽象（文件/目录元数据）
- [ ] File 抽象（打开的文件）
- [ ] Dentry（目录项缓存）
- [ ] Superblock（文件系统元信息）
- [ ] 文件系统 trait 定义

#### 4.2 文件描述符管理
- [ ] FD 表（每个进程）
- [ ] 标准输入/输出/错误（0/1/2）
- [ ] `open()` / `close()`
- [ ] `read()` / `write()`
- [ ] `lseek()` - 文件偏移

#### 4.3 路径解析
- [ ] 绝对路径和相对路径
- [ ] 符号链接处理
- [ ] 路径规范化（. 和 ..）
- [ ] 挂载点（mount points）

#### 4.4 目录操作
- [ ] `mkdir()` - 创建目录
- [ ] `rmdir()` - 删除目录
- [ ] `readdir()` - 读取目录
- [ ] `chdir()` - 改变工作目录
- [ ] `getcwd()` - 获取当前目录

**关键文件**：
- `os/src/fs/vfs.rs` - VFS 核心
- `os/src/fs/inode.rs` - Inode 定义
- `os/src/fs/file.rs` - File 抽象
- `os/src/fs/path.rs` - 路径处理

**VFS Trait 设计**：
```rust
pub trait FileSystem {
    fn root_inode(&self) -> Arc<dyn Inode>;
    fn name(&self) -> &str;
    fn stat(&self) -> FsStat;
}

pub trait Inode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize>;
    fn metadata(&self) -> Metadata;
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>>;
    fn create(&self, name: &str, type_: InodeType) -> Result<Arc<dyn Inode>>;
    fn unlink(&self, name: &str) -> Result<()>;
}
```

---

### 第五阶段：具体文件系统实现（3-4周）
**目标**：实现至少一种真实的文件系统

#### 5.1 选择文件系统类型
**推荐顺序**：
1. **RamFS**（内存文件系统）- 最简单，用于测试
2. **FAT32** - 简单且广泛支持
3. **Ext2** - 类 Unix 文件系统
4. **自定义简单 FS** - 学习目的

#### 5.2 RamFS 实现（最优先）
- [ ] 内存中的 Inode 结构
- [ ] 目录树存储
- [ ] 文件内容存储在堆内存
- [ ] 快速原型验证 VFS 接口

#### 5.3 块设备抽象
- [ ] BlockDevice trait
- [ ] 内存块设备（测试用）
- [ ] VirtIO 块设备驱动
- [ ] 块缓存（Block Cache）

#### 5.4 FAT32 文件系统
- [ ] FAT 表解析
- [ ] 目录项解析
- [ ] 文件读写
- [ ] 长文件名支持（LFN）

#### 5.5 磁盘镜像制作
- [ ] 创建 FAT32 磁盘镜像
- [ ] QEMU 挂载磁盘
- [ ] 从磁盘加载用户程序

**关键文件**：
- `os/src/fs/ramfs.rs` - 内存文件系统
- `os/src/fs/fat32.rs` - FAT32 实现
- `os/src/drivers/block.rs` - 块设备
- `os/src/drivers/virtio_blk.rs` - VirtIO 块设备

**块设备接口**：
```rust
pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<()>;
    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<()>;
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> usize;
}
```

---

### 第六阶段：用户程序加载（2周）
**目标**：能够加载和运行用户态程序

#### 6.1 ELF 解析器
- [ ] 解析 ELF 头
- [ ] 解析程序头（Program Headers）
- [ ] 加载段到内存
- [ ] 设置入口点

#### 6.2 用户态环境
- [ ] 用户栈设置
- [ ] 命令行参数和环境变量
- [ ] 辅助向量（Auxiliary Vector）
- [ ] 初始化用户态上下文

#### 6.3 用户程序示例
- [ ] Hello World
- [ ] 简单 Shell
- [ ] 文件操作测试
- [ ] 进程管理测试

**关键文件**：
- `os/src/loader/elf.rs` - ELF 加载器
- `user/src/bin/*.rs` - 用户程序
- `user/build.rs` - 构建用户程序

---

## 详细实现步骤

### 实现 mkdir 系统调用的完整路径

#### 前置依赖关系图
```
mkdir 系统调用
  └─ 需要文件系统 VFS
      ├─ 需要 Inode 抽象
      ├─ 需要路径解析
      └─ 需要具体 FS 实现（RamFS/FAT32）
          └─ 需要块设备驱动（如果是磁盘 FS）
              └─ 需要 VirtIO 驱动（可选）

  └─ 需要系统调用机制
      ├─ 需要 ecall 处理
      ├─ 需要用户态/内核态切换
      └─ 需要进程管理（当前进程的工作目录）
          ├─ 需要进程控制块（PCB）
          ├─ 需要地址空间管理
          └─ 需要上下文切换
              └─ 需要完善的虚拟内存
```

#### 具体实施顺序

##### 步骤 1：完善虚拟内存（Week 1-2）

**任务清单**：
```rust
// 1. 创建 os/src/memory/paging.rs
// 实现页表遍历
pub fn translate_addr(page_table: &PageTable, vaddr: VirtAddr) -> Option<PhysAddr>;

// 实现页面映射
pub fn map_page(
    page_table: &mut PageTable,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: PageTableFlags,
    allocator: &mut FrameAllocator
) -> Result<()>;

// 实现页面取消映射
pub fn unmap_page(page_table: &mut PageTable, vaddr: VirtAddr) -> Result<PhysFrame>;
```

**测试用例**：
```rust
#[test]
fn test_page_mapping() {
    let mut page_table = PageTable::new();
    let vaddr = VirtAddr::new(0x1000);
    let paddr = PhysAddr::new(0x8000_1000);

    map_page(&mut page_table, vaddr, paddr, PageTableFlags::RW).unwrap();
    assert_eq!(translate_addr(&page_table, vaddr), Some(paddr));
}
```

##### 步骤 2：实现地址空间抽象（Week 2）

```rust
// os/src/memory/address_space.rs
pub struct AddressSpace {
    page_table: PageTable,
    areas: Vec<MemoryArea>,
}

impl AddressSpace {
    // 创建新地址空间
    pub fn new() -> Self;

    // 映射内存区域
    pub fn map_region(&mut self, start: VirtAddr, size: usize, flags: Flags) -> Result<()>;

    // 取消映射
    pub fn unmap_region(&mut self, start: VirtAddr, size: usize) -> Result<()>;

    // 激活此地址空间（写入 satp）
    pub fn activate(&self);

    // 复制地址空间（用于 fork）
    pub fn clone(&self) -> Result<Self>;
}
```

##### 步骤 3：实现基础系统调用机制（Week 3）

```rust
// os/src/syscall/mod.rs
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;
pub const SYS_YIELD: usize = 124;
pub const SYS_GETPID: usize = 172;

pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_EXIT => sys_exit(args[0] as i32),
        SYS_YIELD => sys_yield(),
        SYS_GETPID => sys_getpid(),
        _ => {
            println!("Unknown syscall: {}", syscall_id);
            -1
        }
    }
}
```

**修改中断处理器**：
```rust
// os/src/interrupts.rs
fn syscall_handler(sepc: usize) {
    // 读取系统调用号和参数
    let syscall_id: usize;
    let args: [usize; 6];

    unsafe {
        asm!(
            "mv {}, a7",  // 系统调用号
            "mv {}, a0",  // 参数 0
            // ... 读取 a1-a5
            out(reg) syscall_id,
            out(reg) args[0],
        );
    }

    let result = syscall::syscall(syscall_id, args);

    // 将返回值写入 a0
    unsafe {
        asm!("mv a0, {}", in(reg) result);
    }

    // 跳过 ecall 指令
    sepc::write(sepc + 4);
}
```

##### 步骤 4：实现进程控制块（Week 4）

```rust
// os/src/process/pcb.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct Context {
    pub regs: [usize; 32],  // x0-x31
    pub sepc: usize,
    pub sstatus: usize,
}

pub struct Process {
    pub pid: Pid,
    pub parent: Option<Pid>,
    pub state: ProcessState,
    pub context: Context,
    pub address_space: AddressSpace,
    pub working_dir: String,  // 工作目录（简化版）
    pub exit_code: Option<i32>,
}

impl Process {
    pub fn new(pid: Pid, address_space: AddressSpace) -> Self;
    pub fn switch_to(&mut self, next: &mut Process);
}
```

##### 步骤 5：实现简单调度器（Week 4）

```rust
// os/src/process/scheduler.rs
pub struct Scheduler {
    processes: BTreeMap<Pid, Process>,
    ready_queue: VecDeque<Pid>,
    current: Option<Pid>,
}

impl Scheduler {
    pub fn new() -> Self;
    pub fn add_process(&mut self, process: Process);
    pub fn schedule(&mut self) -> Option<Pid>;
    pub fn yield_current(&mut self);
    pub fn exit_current(&mut self, exit_code: i32);
}
```

##### 步骤 6：实现 VFS 框架（Week 5-6）

```rust
// os/src/fs/vfs.rs
pub trait Inode: Send + Sync {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize>;
    fn metadata(&self) -> Metadata;
    fn inode_type(&self) -> InodeType;

    // 目录操作
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>>;
    fn create(&self, name: &str, type_: InodeType) -> Result<Arc<dyn Inode>>;
    fn unlink(&self, name: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
}

#[derive(Debug, Clone, Copy)]
pub enum InodeType {
    File,
    Directory,
    SymLink,
    CharDevice,
    BlockDevice,
}
```

##### 步骤 7：实现 RamFS（Week 6）

```rust
// os/src/fs/ramfs.rs
pub struct RamFS {
    root: Arc<RamFSInode>,
}

pub struct RamFSInode {
    type_: InodeType,
    name: String,
    content: RwLock<Vec<u8>>,  // 文件内容
    children: RwLock<BTreeMap<String, Arc<RamFSInode>>>,  // 子节点
    metadata: Metadata,
}

impl Inode for RamFSInode {
    fn create(&self, name: &str, type_: InodeType) -> Result<Arc<dyn Inode>> {
        let mut children = self.children.write();

        if children.contains_key(name) {
            return Err(Error::AlreadyExists);
        }

        let new_inode = Arc::new(RamFSInode::new(name.to_string(), type_));
        children.insert(name.to_string(), new_inode.clone());

        Ok(new_inode)
    }
}
```

##### 步骤 8：实现 mkdir 系统调用（Week 7）

```rust
// os/src/syscall/fs.rs
pub const SYS_MKDIR: usize = 34;

pub fn sys_mkdir(path: *const u8, path_len: usize) -> isize {
    let path = unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(path, path_len))
    };

    // 获取当前进程
    let current = PROCESS_MANAGER.current().unwrap();

    // 解析路径
    let (parent_path, dir_name) = parse_path(path);

    // 查找父目录
    let parent_inode = match lookup_path(&current.working_dir, parent_path) {
        Ok(inode) => inode,
        Err(e) => return error_to_errno(e),
    };

    // 创建目录
    match parent_inode.create(dir_name, InodeType::Directory) {
        Ok(_) => 0,  // 成功
        Err(e) => error_to_errno(e),
    }
}

// 路径解析
fn parse_path(path: &str) -> (&str, &str) {
    if let Some(pos) = path.rfind('/') {
        (&path[..pos], &path[pos+1..])
    } else {
        (".", path)
    }
}

// 路径查找
fn lookup_path(cwd: &str, path: &str) -> Result<Arc<dyn Inode>> {
    let fs = get_root_fs();
    let mut current = if path.starts_with('/') {
        fs.root_inode()
    } else {
        lookup_path("/", cwd)?
    };

    for component in path.split('/').filter(|s| !s.is_empty()) {
        if component == "." {
            continue;
        } else if component == ".." {
            // TODO: 向上查找
            continue;
        } else {
            current = current.lookup(component)?;
        }
    }

    Ok(current)
}
```

##### 步骤 9：用户态封装（Week 7）

```rust
// user/src/syscall.rs
pub fn mkdir(path: &str) -> Result<()> {
    let ret = syscall(
        SYS_MKDIR,
        [path.as_ptr() as usize, path.len(), 0, 0, 0, 0]
    );

    if ret == 0 {
        Ok(())
    } else {
        Err(Error::from_errno(-ret as i32))
    }
}

// 用户程序示例
// user/src/bin/test_mkdir.rs
#![no_std]
#![no_main]

use user::*;

#[no_mangle]
fn main() -> i32 {
    println!("Testing mkdir...");

    if let Err(e) = mkdir("/test_dir") {
        println!("mkdir failed: {:?}", e);
        return -1;
    }

    println!("Directory created successfully!");
    0
}
```

---

## 系统调用实现优先级

### Phase 1: 核心系统调用（最优先）
```
1. sys_write     - 输出（用于调试）
2. sys_read      - 输入
3. sys_exit      - 进程退出
4. sys_yield     - 主动调度
5. sys_getpid    - 获取进程 ID
```

### Phase 2: 进程管理
```
6. sys_fork      - 创建进程
7. sys_exec      - 执行程序
8. sys_wait      - 等待子进程
9. sys_kill      - 发送信号
10. sys_sleep    - 睡眠
```

### Phase 3: 文件系统基础
```
11. sys_open     - 打开文件
12. sys_close    - 关闭文件
13. sys_read     - 读文件（复用）
14. sys_write    - 写文件（复用）
15. sys_lseek    - 移动文件指针
```

### Phase 4: 目录操作
```
16. sys_mkdir    - 创建目录 ⭐ 目标
17. sys_rmdir    - 删除目录
18. sys_chdir    - 改变目录
19. sys_getcwd   - 获取当前目录
20. sys_opendir  - 打开目录
21. sys_readdir  - 读目录
```

### Phase 5: 高级文件操作
```
22. sys_stat     - 文件信息
23. sys_fstat    - 文件描述符信息
24. sys_link     - 硬链接
25. sys_unlink   - 删除文件
26. sys_rename   - 重命名
27. sys_chmod    - 修改权限
```

---

## 开发建议和注意事项

### 1. 渐进式开发
- 每完成一个模块，立即编写测试
- 优先实现最简单的版本，再逐步优化
- 保持代码可编译、可运行

### 2. 测试驱动
```rust
// 示例：先写测试
#[test]
fn test_mkdir() {
    let fs = RamFS::new();
    let root = fs.root_inode();

    // 创建目录
    root.create("test", InodeType::Directory).unwrap();

    // 验证目录存在
    let test_dir = root.lookup("test").unwrap();
    assert_eq!(test_dir.inode_type(), InodeType::Directory);
}
```

### 3. 调试技巧
- 充分使用 `serial_println!` 输出调试信息
- 为每个模块添加详细日志
- 使用 GDB 调试（QEMU -s -S）

### 4. 代码组织
```
os/src/
├── memory/
│   ├── mod.rs
│   ├── paging.rs          # 页表管理
│   ├── address_space.rs   # 地址空间
│   └── frame_allocator.rs # 帧分配器
├── process/
│   ├── mod.rs
│   ├── pcb.rs             # 进程控制块
│   ├── scheduler.rs       # 调度器
│   └── context.rs         # 上下文切换
├── syscall/
│   ├── mod.rs
│   ├── process.rs         # 进程相关
│   └── fs.rs              # 文件系统相关
└── fs/
    ├── mod.rs
    ├── vfs.rs             # VFS 抽象
    ├── ramfs.rs           # 内存文件系统
    └── fat32.rs           # FAT32（可选）
```

### 5. 参考资源
- **rCore-Tutorial**: https://rcore-os.github.io/rCore-Tutorial-Book-v3/
- **xv6-riscv**: https://github.com/mit-pdos/xv6-riscv
- **Writing an OS in Rust**: https://os.phil-opp.com/
- **RISC-V Spec**: https://riscv.org/technical/specifications/

---

## 时间线总结

| 阶段 | 时间 | 里程碑 | 可实现的系统调用 |
|------|------|--------|------------------|
| **第 1 阶段** | Week 1-3 | 虚拟内存完成 | - |
| **第 2 阶段** | Week 4 | 系统调用机制 | write, exit, yield, getpid |
| **第 3 阶段** | Week 5-7 | 进程管理 | fork, exec, wait |
| **第 4 阶段** | Week 8-9 | VFS + RamFS | open, close, read, write |
| **第 5 阶段** | Week 10 | 目录操作 | **mkdir**, chdir, getcwd ⭐ |
| **第 6 阶段** | Week 11-12 | 用户程序 | 完整的用户态程序 |

---

## 下一步行动

### 立即开始（本周）
1. 创建 `os/src/memory/paging.rs` 文件
2. 实现基础的页表遍历函数
3. 编写页表单元测试

### 第一个里程碑（3周内）
- 完成虚拟内存管理
- 实现地址空间抽象
- 能够创建和切换地址空间

### 最终目标（3个月内）
- 实现完整的 `mkdir` 系统调用
- 能够运行用户态程序创建目录
- 支持基本的文件系统操作

---

**记住**：操作系统开发是一个长期过程，不要急于求成。每个模块都需要仔细设计和充分测试。遇到问题时，参考成熟的操作系统（如 xv6、rCore）的实现方式。

祝开发顺利！🚀
