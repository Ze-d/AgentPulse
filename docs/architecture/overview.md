# Architecture Overview

## Tech Stack

| 层 | 技术 | 用途 |
|---|------|------|
| Desktop Shell | Tauri 2 | 窗口管理、系统托盘、Rust 运行时 |
| Frontend | Vue 3 + TypeScript + Tailwind CSS | 悬浮窗 UI |
| State | Pinia | 前端状态管理，2s 轮询 |
| Backend | Rust (agentpulse_lib) | HTTP 服务器、状态机、SQLite、进程监控 |
| HTTP Server | tiny_http 0.12 | 内嵌 HTTP，端口 17878 |
| Database | SQLite (rusqlite 0.31, in-memory) | 事件 + session 临时存储，重启后清空 |
| Process Monitor | sysinfo 0.31 | 跨平台进程存活检测，跳过终态 session（completed/failed） |
| Adapter | Python 3 | Claude Code hook stdin → HTTP 转发 + 进程树遍历 |
| IPC | Tauri invoke/events | Rust ↔ Vue 数据流 |

## Architecture: 4 Layers

```
┌─────────────────────────────────────────┐
│  Floating Window (Vue 3 + Tailwind)     │  ← Tauri invoke (IPC)
├─────────────────────────────────────────┤
│  Monitor Core (Rust)                    │  ← HTTP :17878 + SQLite + state machine
├─────────────────────────────────────────┤
│  Hooks In (Python)  │  File In  │ ... │  ← v1: Hooks only
├─────────────────────────────────────────┤
│  Agents (Claude Code CLI)               │
└─────────────────────────────────────────┘
```

## Key Design Decisions

1. **内存数据库** — `Database::new_in_memory()` 每次启动干净状态，只关注当前活跃的 Claude Code session。`Database::new(path)` 文件持久化方法已实现保留备用
2. **HTTP 作为适配器协议** — Python hook 脚本通过 HTTP POST 与 Rust 后端通信，解耦语言和进程边界
3. **共享 Arc<Mutex<Database>>** — 事件服务器线程和 Tauri command handler 共享同一数据库实例
4. **前端轮询而非 WebSocket** — 简单可靠，2s 间隔对单用户本地场景足够
5. **无边框置顶窗口** — `decorations: false` + `alwaysOnTop: true` + `transparent: true` + `shadow: false`，最小尺寸 280x72，自适应高度最大 420px，12px 圆角，等宽字体终端风格
6. **Python 适配器保持轻量** — 无第三方依赖，仅使用标准库 (json, urllib, argparse, logging)
7. **Session 生命周期展示 + 进程存活检测** — 活跃 session 卡片实时显示状态，完成后保留 "done" 标识，继续对话自动恢复。monitor_hook.py 通过 Windows `CreateToolhelp32Snapshot` API 向上遍历进程树（跳过 cmd.exe / powershell.exe 等 shell 中间层）获取 CC 的 node.exe 真实 PID。Rust 后台线程 (process_checker) 每 5 秒通过 sysinfo 检测 PID 是否存活：活跃 session PID 死亡 → 自动删除；Completed/Failed 等终态 session 跳过删除，保留展示。终态 session 收到新事件时状态机自动恢复（PreToolUse → ToolRunning，其他 → Running）。无 PID 的 session（如 SessionStart 丢失）不会被自动清理，需重启应用清除

## Data Flow

```
Claude Code hook 触发
  → settings.json 中的 hook 配置
    → 执行 monitor_hook.py (stdin 传入 hook JSON)
      → _walk_process_tree_to_cc() 向上遍历进程树获取 CC 真实 PID
      → 注入 process_pid 到 event payload
      → POST http://127.0.0.1:17878/api/events
        → normalize_claude_code_event() 提取 + 映射
          → StateMachine::transition() 验证状态流转
            → db.upsert_session() + db.insert_event()
              → Tauri get_sessions command → db.list_all_sessions()
                → Vue store 2s 轮询 → UI 展示全部 session（含 completed "done" 标识）
                  process_checker.rs (5s 轮询):
                    → db.list_sessions_with_pid() → is_active_status() 过滤终态
                      → sysinfo 检查 PID 存活 → 非终态 + PID 死亡 → db.delete_session()
                      → 终态 (Completed/Failed) 保留展示，重启应用清空
```

## Port Allocation

| 端口 | 用途 |
|------|------|
| 1420 | Vite dev server (前端热更新) |
| 17878 | AgentPulse HTTP event server |
