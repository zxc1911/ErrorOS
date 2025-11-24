# Git Commit Message

## Commit Title
```
feat: 实现 RISC-V Sv39 虚拟内存系统（可视化教学版）
```

## Commit Body
```
实现完整的三级页表机制和地址空间管理，包含丰富的可视化教学特性。

## 新增功能
- 实现 Sv39 三级页表遍历（walk_page_table）
- 实现页面映射和取消映射（map_page/unmap_page）
- 实现地址空间抽象（AddressSpace）
- 创建并激活内核地址空间
- 支持恒等映射（用于内核代码段）
- 添加详细的可视化输出（教学特色）

## 文件变更
- 新增: os/src/memory/paging.rs - 页表管理核心功能
- 新增: os/src/memory/address_space.rs - 地址空间抽象
- 重构: os/src/memory.rs -> os/src/memory/mod.rs - 模块化
- 更新: os/src/main.rs - 添加虚拟内存测试
- 新增: docs/01_virtual_memory_implementation.md - 技术文档

## 教学特色
- 页表遍历过程可视化（逐级展示 VPN 查找）
- 地址空间布局表格显示
- 映射过程详细日志输出
- 丰富的代码注释和教学说明

## 测试验证
- ✅ 页表映射和遍历功能正常
- ✅ 地址转换结果正确
- ✅ 虚拟内存激活后系统稳定运行
- ✅ 所有可视化输出清晰易懂

## 技术细节
- 支持 4KB 页面大小
- 使用 sfence.vma 指令刷新 TLB
- 按需分配中间页表（节省内存）
- 实现恒等映射机制

## 下一步计划
- 实现系统调用机制
- 添加进程管理功能
- 完善用户态地址空间支持

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>
```

## Commit Command
```bash
cd /Users/weisiyang/Blog_OS
git add os/src/memory/
git add os/src/main.rs
git add docs/01_virtual_memory_implementation.md
git commit -m "$(cat <<'EOF'
feat: 实现 RISC-V Sv39 虚拟内存系统（可视化教学版）

实现完整的三级页表机制和地址空间管理，包含丰富的可视化教学特性。

## 新增功能
- 实现 Sv39 三级页表遍历（walk_page_table）
- 实现页面映射和取消映射（map_page/unmap_page）
- 实现地址空间抽象（AddressSpace）
- 创建并激活内核地址空间
- 支持恒等映射（用于内核代码段）
- 添加详细的可视化输出（教学特色）

## 文件变更
- 新增: os/src/memory/paging.rs - 页表管理核心功能
- 新增: os/src/memory/address_space.rs - 地址空间抽象
- 重构: os/src/memory.rs -> os/src/memory/mod.rs - 模块化
- 更新: os/src/main.rs - 添加虚拟内存测试
- 新增: docs/01_virtual_memory_implementation.md - 技术文档

## 教学特色
- 页表遍历过程可视化（逐级展示 VPN 查找）
- 地址空间布局表格显示
- 映射过程详细日志输出
- 丰富的代码注释和教学说明

## 测试验证
- ✅ 页表映射和遍历功能正常
- ✅ 地址转换结果正确
- ✅ 虚拟内存激活后系统稳定运行
- ✅ 所有可视化输出清晰易懂

🤖 Generated with Claude Code

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```
