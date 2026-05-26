# Architecture Overview

## Tech Stack

| 层 | 技术 | 用途 |
|---|------|------|
| Desktop Shell | Tauri 2 | 窗口管理、系统托盘、Rust 运行时 |
| Frontend | Vue 3 + TypeScript + Tailwind CSS | 悬浮窗 UI |
| State | Pinia | 前端状态管理，2s 轮询 |
| Backend | Rust (agentpulse_lib) | HTTP 服务器、状态机、SQLite、进程监控 |
| HTTP Server | tiny_http 0.12 | 内嵌 HTTP，端口 17878 |
| Database | SQLite (rusqlite 0.31, bundled) | 事件 + session 持久化 |
| Process Monitor | sysinfo 0.31 | 跨平台进程存活检测（PID → 自动清理 session） |
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

1. **内存数据库用于开发** — `Database::new_in_memory()` 简化测试，每次启动干净状态。生产环境应改为文件持久化
2. **HTTP 作为适配器协议** — Python hook 脚本通过 HTTP POST 与 Rust 后端通信，解耦语言和进程边界
3. **共享 Arc<Mutex<Database>>** — 事件服务器线程和 Tauri command handler 共享同一数据库实例
4. **前端轮询而非 WebSocket** — 简单可靠，2s 间隔对单用户本地场景足够
5. **无边框置顶窗口** — `decorations: false` + `alwaysOnTop: true` + `transparent: true` + `shadow: false`，最小尺寸 280x72，自适应高度最大 420px，12px 圆角，等宽字体终端风格
6. **Python 适配器保持轻量** — 无第三方依赖，仅使用标准库 (json, urllib, argparse, logging)
7. **Session 持久展示 + 进程存活检测** — 完成任务后卡片保留在面板，面板卡片数与 CC 终端数一一对应。monitor_hook.py 通过 Windows `CreateToolhelp32Snapshot` API 向上遍历进程树（跳过 cmd.exe / powershell.exe 等 shell 中间层）获取 CC 的 node.exe 真实 PID。Rust 后台线程 (process_checker) 每 5 秒通过 sysinfo 检测 PID 是否存活，进程退出后自动删除对应 session，卡片随之消失。无 PID 的 session（如 SessionStart 丢失）不会被自动清理，需重启应用清除

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
              → Tauri get_sessions command → db.list_all_sessions() (不过滤终态)
                → Vue store 2s 轮询 → UI 展示全部 session
                  process_checker.rs (5s 轮询):
                    → db.list_sessions_with_pid() → sysinfo 检查 PID 存活
                      → PID 死亡 → db.delete_session() → card 自动消失
```

## Port Allocation

| 端口 | 用途 |
|------|------|
| 1420 | Vite dev server (前端热更新) |
| 17878 | AgentPulse HTTP event server |
