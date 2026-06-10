# 07 — Hook 适配器去 Python 依赖

**状态：** 已完成  
**优先级：** 中  
**创建日期：** 2026-06-10  
**完成日期：** 2026-06-10

## 问题

当前 `monitor_hook.py` 适配器依赖外部 Python 解释器。虽然 `resolve_python()` 已改为使用 `sys.executable` 绝对路径（兜底），但根本问题未解决：

1. **部分用户的电脑未安装 Python** — AgentPulse 无法注册 hook
2. **跨环境 PATH 不一致** — AgentPulse 启动时的 PATH 与 target agent（Codex App Server、CC 等）运行时的 PATH 不同，即使用绝对路径也可能因权限/沙箱限制而失败
3. **启动延迟** — 每个 hook 事件触发都需要启动一个 Python 进程（~100-500ms），高频事件时开销显著

## 候选方案

### 方案 A：Rust 编译为独立可执行文件

将 `monitor_hook.py` 的逻辑用 Rust 重写，编译为独立 `.exe`，打包进 Tauri resource：

- 零外部依赖
- 启动速度远快于 Python（<10ms vs ~200ms）
- 通过 Tauri resource 机制分发，自动更新
- **代价**：需要处理跨平台编译（Windows x64/arm64, macOS, Linux）

### 方案 B：HTTP 长连接 / Unix Socket

AgentPulse 启动时打开一个本地 socket，agent hook 直接写 JSON 到 socket 而非通过 Python 脚本 POST：

- Claude Code 支持 `"type": "socket"` hook（待验证）
- Codex 当前只支持 `"type": "command"` hook（需关注上游更新）
- **代价**：依赖各 agent 的 hook 机制支持，目前不统一

### 方案 C：内嵌 Python（pyo3）

在 Rust 二进制中嵌入 Python 运行时，hook 事件直接调用内嵌解释器：

- 无需外部 Python 安装
- **代价**：显著增加二进制体积（~30MB+），打包复杂度高

### 方案 D：混合方案（短期推荐）

- 保留当前 Python 适配器作为默认路径
- 用 Rust 编译一个轻量 `agentpulse-hook.exe`（~2MB），作为备选
- `install_hooks` 时检测 Python 是否可用，不可用时自动 fallback 到 Rust 二进制
- 用户也可通过 config 手动选择适配器类型

## 关联

- [[06-ux-enhancements]] — 与 Codex/Gemini/Copilot 适配器相关
- Codex 集成设计文档：`docs/superpowers/specs/2026-06-10-codex-integration-design.md`

## 当前兜底逻辑（2026-06-10）

`resolve_python()` in `apps/desktop/src-tauri/src/hooks.rs`:

- 通过 `python3 -c "import sys; print(sys.executable)"` 获取绝对路径
- 写入 TOML/JSON hook 配置时使用绝对路径而非 `python3`
- 如果连这一步也失败，fallback 到 `"python"` 字符串（此时 hook 大概率无法工作）
