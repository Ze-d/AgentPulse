# AgentPulse Codex 集成设计文档

**日期：** 2026-06-10
**状态：** 已批准

## 概述

在现有 Claude Code 集成基础上，新增 OpenAI Codex CLI 支持。Codex CLI 的 hook 机制与 Claude Code 高度同构 — 都是通过配置文件中注册 command 类型 hook，stdin 传入 JSON 事件 — 因此改动范围很小。

用户可在同一悬浮窗中同时看到 Claude Code 和 Codex session 的实时状态卡片。

## 调研结论

### Codex CLI Hook 机制

| 维度 | Claude Code | Codex CLI |
|------|------------|-----------|
| 配置文件 | `~/.claude/settings.json` (JSON) | `~/.codex/config.toml` (TOML) |
| Hook 声明 | `hooks.SessionStart = [{...}]` | `[hooks] SessionStart = [...]` |
| 事件传输 | command → stdin JSON | command → stdin JSON |
| 事件字段 | `session_id`, `cwd`, `hook_event_name`, `transcript_path` | **相同**，额外有 `model`, `permission_mode`, `source`, `turn_id` |
| 进程 | `node.exe` | `codex.exe` (Rust 二进制) |

### 事件覆盖

Codex 共 10 个 hook 事件，AgentPulse 订阅其中 6 个核心事件：

| Codex 事件 | AgentPulse 事件类型 | AgentPulse 状态 |
|-----------|-------------------|----------------|
| SessionStart | session_start | starting |
| PreToolUse | pre_tool_use | tool_running |
| PostToolUse | post_tool_use | running |
| PermissionRequest | permission_request | waiting_permission |
| UserPromptSubmit | notification | running |
| Stop | stop | completed |

不订阅的 4 个：PreCompact、PostCompact、SubagentStart、SubagentStop（v1.0 后再考虑）。

## 架构

```
Claude Code 会话               Codex CLI 会话
    │                              │
    ▼                              ▼
monitor_hook.py (复用)     monitor_hook.py (复用/符号链接)
    │                              │
    ▼                              ▼
POST /api/events             POST /api/events
    │                              │
    ▼                              ▼
normalize_claude_code_event()  normalize_codex_event()  ← 新增
    │                              │
    └──────────┬───────────────────┘
               ▼
        handle_event() (通用)
               │
               ▼
          state_machine (通用)
               │
               ▼
          SQLite (source 字段区分)
```

## 改动清单

### 1. 新建 `adapters/codex/install_hooks.py`

负责读写 `~/.codex/config.toml`：

```toml
[hooks]
SessionStart = [
  { matcher = "", hooks = [
    { type = "command", command = "python \"...monitor_hook.py\"" }
  ]}
]
PreToolUse = [ ... ]
PostToolUse = [ ... ]
PermissionRequest = [ ... ]
UserPromptSubmit = [ ... ]
Stop = [ ... ]
```

功能：
- `--install`：合并 hook 配置到现有 config.toml（保留已有配置）
- `--remove`：移除 AgentPulse 注册的 6 个 hook
- `--status`：显示安装状态
- `--dry-run`：预览改动

### 2. 更新 `adapters/codex/monitor_hook.py`

复用 CC 版本的 `monitor_hook.py`，因为 stdin JSON 格式兼容。如需区分来源，在 POST 前注入 `"agent_source": "codex"` 字段，或通过 HTTP header `X-Agent-Source: codex` 传递。

**决策**：通过新增 JSON 字段 `"agent_source": "codex"` 来标识来源，这样不需要改动 HTTP 协议。

### 3. 更新 `event_server.rs`

新增 `normalize_codex_event()` 函数，将 Codex 的 stdin JSON 转换为 `AgentEvent`：

- `hook_event_name` → `EventType` 映射与 CC 一致
- `source` 设为 `AgentSource::Codex`
- 忽略 Codex 独有字段（`model`, `permission_mode`, `turn_id`）
- PID 来自适配器注入的 `process_pid`

更新请求路由逻辑：根据 `agent_source` 字段选择规范化函数。默认为 `ClaudeCode`（向后兼容）。

### 4. 更新 `hooks.rs`

新增 Codex TOML 配置管理：

- `find_codex_config_path()` — 解析 `~/.codex/config.toml`
- `ensure_codex_hooks_installed()` — 幂等安装，类似 CC 版本
- `get_codex_hook_status()` — 查询安装状态
- TOML 解析使用 `toml` crate

新增 Codex monitor script 提取逻辑（与 CC 共用 `extract_monitor_script`）。

### 5. 更新 `lib.rs`

启动时并行安装 CC + Codex hooks：

```rust
// 现有：ensure_hooks_installed for CC
// 新增：ensure_codex_hooks_installed for Codex
```

### 6. PID 探测更新

`monitor_hook.py` 中的 `_walk_process_tree_to_cc()` 重命名为 `_find_agent_pid()`，不再只查找 `node.exe`，而是匹配已知的 agent 进程名列表（`node.exe`, `codex.exe`, `gemini`, `copilot` 等），或简单地返回第一个非 shell 父进程。

### 7. 前端

无需改动。`AgentSource::Codex`、`sourceAbbr("codex")` → `"cx"`、状态卡片颜色等均已就位。

## 不改的内容

- 状态机（`state_machine.rs`）— 通用逻辑，不区分来源
- 数据库（`db.rs`）— `source` 字段已支持 `"codex"`
- 进程检查器（`process_checker.rs`）— 通用逻辑
- 前端组件 — 已通过 `source` 字段区分 agent 类型

## 测试计划

| 层 | 测试 |
|---|------|
| Rust 单元 | `normalize_codex_event` 所有 6 种事件映射 |
| Rust 单元 | Codex TOML 配置的读/写/合并 |
| Rust 单元 | `handle_event` 对 `AgentSource::Codex` 事件的路由 |
| Python 单元 | `install_hooks.py` TOML 生成正确性 |
| 集成测试 | curl → POST /api/events → 验证 source=codex 的 session 被正确创建 |

## 配置

AgentPulse `config.json` 无需新增字段。Codex 安装路径自动检测（`$HOME/.codex/config.toml`）。
