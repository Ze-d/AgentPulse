# Architecture Overview

## Tech Stack

| 层 | 技术 | 用途 |
|---|------|------|
| Desktop Shell | Tauri 2 | 窗口管理、系统托盘、Rust 运行时 |
| Frontend | Vue 3 + TypeScript | 悬浮窗 UI |
| State | Pinia | 前端状态管理，轮询 |
| Styling | Catppuccin Mocha + 等宽字体 | 终端风格配色 |
| Backend | Rust (agentpulse_lib) | HTTP 服务器、状态机、SQLite、进程监控、配置管理 |
| HTTP Server | tiny_http 0.12 | 内嵌 HTTP，端口可配置（默认 17878） |
| Database | SQLite (rusqlite 0.31) | 内存 / 文件持久化，重启后清空或保留 |
| Process Monitor | sysinfo 0.31 | 跨平台进程存活检测，跳过终态 session |
| Logging | tracing + tracing-subscriber | stderr 文本 + JSON 文件轮转 |
| Config | config.json + 环境变量 | 配置文件为主，环境变量覆盖 |
| Adapter | Python 3 | Claude Code hook stdin → HTTP 转发 + 进程树遍历 |
| IPC | Tauri invoke | Rust ↔ Vue 数据流 |

## Architecture: 4 Layers

```
┌─────────────────────────────────────────┐
│  Floating Window (Vue 3 + Pinia)        │  ← Tauri invoke (IPC)
├─────────────────────────────────────────┤
│  Monitor Core (Rust)                    │  ← HTTP :port + SQLite + state machine
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│  │ config   │ │ logging  │ │ state   │ │
│  │ (JSON)   │ │ (tracing)│ │ machine │ │
│  └──────────┘ └──────────┘ └─────────┘ │
├─────────────────────────────────────────┤
│  Hooks In (Python)  │  File In  │ ...  │  ← v1: Hooks only
├─────────────────────────────────────────┤
│  Agents (Claude Code / Codex / Gemini)  │
└─────────────────────────────────────────┘
```

## Key Design Decisions

1. **配置文件驱动** — 端口、轮询间隔、Python 解释器等通过 `config.json` 管理，首次启动自动生成默认配置。环境变量可覆盖（CI 友好）
2. **数据库双模式** — 当前使用 `Database::new_in_memory()` 每次启动干净状态。`Database::new(path)` 文件持久化方法已实现，可配置切换
3. **HTTP 作为适配器协议** — Python hook 脚本通过 HTTP POST 与 Rust 后端通信，解耦语言和进程边界
4. **共享 Arc<Mutex<Database>>** — 事件服务器线程和 Tauri command handler 共享同一数据库实例
5. **前端轮询而非 WebSocket** — 简单可靠，间隔可配置（默认 2s），单用户本地场景足够
6. **无边框置顶窗口** — `decorations: false` + `alwaysOnTop: true` + `transparent: true`，最小尺寸 280x72，自适应高度最大 420px，12px 圆角，等宽字体终端风格
7. **Python 适配器保持轻量** — 无第三方依赖，仅使用标准库 (json, urllib, argparse, logging)
8. **结构化日志** — tracing 输出到 stderr（开发，可读文本）和 JSON 文件（持久化诊断，按小时轮转，自动清理 7 天前日志）
9. **Session 生命周期展示 + 进程存活检测** — 活跃 session 卡片实时显示状态，完成后保留 "done" 标识，继续对话自动恢复。monitor_hook.py 通过 Windows `CreateToolhelp32Snapshot` API 向上遍历进程树获取 CC 真实 PID。Rust 后台线程定期检测 PID 存活：活跃 session PID 死亡 → 自动删除；Completed/Failed 等终态 session 跳过删除，保留展示

## Data Flow

```
Claude Code hook 触发
  → settings.json 中的 hook 配置
    → 执行 monitor_hook.py (stdin 传入 hook JSON)
      → _walk_process_tree_to_cc() 向上遍历进程树获取 CC 真实 PID
      → 注入 process_pid 到 event payload
      → POST http://127.0.0.1:{port}/api/events
        → normalize_claude_code_event() 提取 + 映射
          → StateMachine::transition() 验证状态流转
            → db.upsert_session() + db.insert_event()
              → Tauri get_sessions command → db.list_all_sessions()
                → Vue store 轮询 → UI 展示全部 session（含 completed "done" 标识）
                  process_checker.rs (可配置间隔):
                    → db.list_sessions_with_pid() → is_active_status() 过滤终态
                      → sysinfo 检查 PID 存活 → 非终态 + PID 死亡 → db.delete_session()
                      → 终态 (Completed/Failed) 保留展示，重启应用清空
```

## Module Map

```
lib.rs (入口)
├── config.rs         — 配置加载 (config.json + 环境变量覆盖)
├── logging.rs        — tracing 初始化 (stderr + JSON 文件)
├── db.rs             — SQLite CRUD + cleanup
├── state_machine.rs  — 状态转换 + needs_attention
├── event_server.rs   — HTTP 服务器 (tiny_http)
├── process_checker.rs — 进程存活检测
├── hooks.rs          — Hook 配置管理
├── commands.rs       — Tauri IPC 命令
└── tray.rs           — 系统托盘
```

## Port Allocation

| 端口 | 用途 |
|------|------|
| 1420 | Vite dev server (前端热更新) |
| 17878 | AgentPulse HTTP event server (默认，可通过 config.json 修改) |
