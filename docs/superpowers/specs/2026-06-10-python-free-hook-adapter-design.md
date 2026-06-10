# Python-Free Hook Adapter Design

**日期：** 2026-06-10  
**状态：** 已确认  
**关联：** [[07-python-free-hook-adapter|todo]], [[2026-06-10-codex-integration-design|Codex 集成]]

## 目标

完全移除 AgentPulse 的 Python 依赖，将 4 个 Python 文件替换为单个 Rust 编译的独立二进制 `agentpulse-hook`。

## 动机

1. **用户未安装 Python** — hook 注册失败，AgentPulse 无法工作
2. **跨环境 PATH 不一致** — 即使用 `sys.executable` 绝对路径也可能因沙箱/权限失败
3. **启动延迟** — 每个 hook 事件触发 Python 进程启动（~200-500ms），高频事件开销显著
4. **代码重复** — `monitor_hook.py` ×2 和 `install_hooks.py` ×2 存在 ~90% 重复

## 设计

### 单一二进制：`agentpulse-hook`

一个 Rust 二进制覆盖所有 4 个 Python 文件的功能。

**默认模式（无参数）：hook 事件处理**

```
echo '{"session_id":"...","hook_event_name":"SessionStart",...}' | agentpulse-hook
```

1. 从 stdin 读取 hook JSON
2. 通过 `sysinfo` 遍历进程树检测 agent 来源（CC / Codex / Gemini / Copilot）
3. POST 到 `http://127.0.0.1:17878/api/events`（重试 3 次，间隔 1s）
4. 以 exit code 0（成功）或 1（失败）退出

支持标志：
- `--test`：输出 enriched JSON 到 stdout，不发送 HTTP 请求（调试用）
- `--version`：打印版本号并退出

**子命令模式：hook 安装管理**

```
agentpulse-hook install   [--claude|--codex] [--force] [--path <file>]
agentpulse-hook remove    [--claude|--codex] [--path <file>]
agentpulse-hook status    [--claude|--codex] [--path <file>]
agentpulse-hook dry-run   [--claude|--codex] [--path <file>]
```

- `--claude`（默认）：操作 `~/.claude/settings.json`（JSON 格式）
- `--codex`：操作 `~/.codex/config.toml`（TOML 格式）
- `--path`：覆盖默认路径
- `--force`：强制覆盖已有 hooks

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `AGENTPULSE_URL` | `http://127.0.0.1:17878/api/events` | 事件服务器地址 |
| `AGENTPULSE_TIMEOUT` | `5` | HTTP 请求超时（秒） |
| `AGENTPULSE_LOG_LEVEL` | `INFO` | 日志级别（stderr） |

### 二进制属性

| 属性 | 目标 |
|------|------|
| 大小 | <1.5MB（release + strip） |
| 启动时间 | <10ms |
| 外部依赖 | 零（静态链接） |
| 平台 | Windows x64, macOS arm64/x64, Linux x64 |

### 依赖

所有依赖已存在于项目 `Cargo.toml` 中，或为极轻量 crate：
- `ureq` — 阻塞 HTTP 客户端（~50KB）
- `clap` (derive) — CLI 参数解析
- `serde` / `serde_json` — JSON（已有）
- `toml` — TOML（已有）
- `sysinfo` — 跨平台进程检测（已有）
- `log` / `env_logger` — 日志（已有）

### 进程检测策略（跨平台）

- **Windows**：`sysinfo::System::new_all()` + `process.parent()` 遍历，过滤 shell wrapper（cmd.exe, powershell.exe）
- **macOS**：同上
- **Linux**：同上
- 兜底：返回当前进程的 ppid，agent_source 设为 `"unknown"`

### 与现有 `hooks.rs` 的关系

`hooks.rs` 保留原有 Rust 实现（`ensure_hooks_installed`、`ensure_codex_hooks_installed` 等），由 Tauri app 内部调用。仅做以下修改：

1. **删除** `resolve_python()` — 不再需要查找 Python 解释器
2. **重命名** `find_monitor_script()` → `find_hook_binary()` — 查找 `agentpulse-hook`（Windows 加 `.exe`）
3. **重命名** `extract_monitor_script()` → `extract_hook_binary()` — 复制二进制而非 `.py`
4. **修改** `build_hook_configs()` — command 从 `python3 /path/monitor_hook.py` 变为 `/path/agentpulse-hook`
5. **修改** `build_codex_hook_configs()` — 同上
6. **更新** `ensure_hooks_installed()` / `ensure_codex_hooks_installed()` 签名去掉 `python` 参数

### 调用者适配

- `commands.rs::install_hooks_cmd()` — 去掉 `resolve_python()` 调用
- `lib.rs` 启动时 auto-install — 同上
- Codex 对应函数同理

## 文件变更汇总

| 文件 | 动作 |
|------|------|
| `adapters/hook-adapter/` | **新建** — Rust 二进制 crate |
| `adapters/claude-code/monitor_hook.py` | **删除** |
| `adapters/claude-code/install_hooks.py` | **删除** |
| `adapters/codex/monitor_hook.py` | **删除** |
| `adapters/codex/install_hooks.py` | **删除** |
| `apps/desktop/src-tauri/src/hooks.rs` | **修改** — 去 Python 化 |
| `apps/desktop/src-tauri/src/commands.rs` | **修改** — 适配新签名 |
| `apps/desktop/src-tauri/src/lib.rs` | **修改** — 适配新签名 |
| `apps/desktop/src-tauri/tauri.conf.json` | **修改** — resources 指向二进制 |
| `apps/desktop/src-tauri/Cargo.toml` | **修改** — 添加 workspace member 或 dependency |
| `tests/unit/test_install_hooks.py` | **删除** — 迁移到 Rust 测试 |
| `tests/unit/test_monitor_hook.py` | **删除** — 迁移到 Rust 测试 |
| README.md | **修改** — 更新命令示例 |
| `docs/local-development-guide.md` | **修改** — 更新命令示例 |
| `docs/flows/02-hooks-installation.md` | **修改** — 移除 Python 引用 |
| `docs/architecture/module-boundaries.md` | **修改** — 更新模块说明 |
| `docs/testing/testing-strategy.md` | **修改** — 更新测试策略 |
| `AGENTS.md` | **修改** — 更新文件列表 |
| `docs/ai/context-map.md` | **修改** — 更新文件列表 |
| `docs/todos/07-python-free-hook-adapter.md` | **修改** — 更新状态 |

## 实现顺序

1. 创建 `adapters/hook-adapter/` crate，实现 hook 事件处理（默认模式）
2. 实现 CLI 子命令（install/remove/status/dry-run）
3. 修改 `hooks.rs`（去 `resolve_python`，适配新二进制路径）
4. 修改 `commands.rs` 和 `lib.rs` 调用方
5. 更新 `tauri.conf.json` 打包配置
6. 编写 Rust 测试（替代 Python 测试）
7. 删除 4 个 Python 文件
8. 更新所有文档
