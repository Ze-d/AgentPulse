# AgentPulse 功能流程文档

本目录包含 AgentPulse 项目各核心功能的详细流程图和说明。

## 流程索引

| 编号 | 流程 | 文档 | 涉及模块 |
|------|------|------|----------|
| 1 | 应用启动 | [01-app-startup.md](01-app-startup.md) | `lib.rs`, `main.rs`, `hooks.rs`, `event_server.rs`, `process_checker.rs` |
| 2 | Hooks 安装与配置 | [02-hooks-installation.md](02-hooks-installation.md) | `hooks.rs`, `install_hooks.py`, `commands.rs` |
| 3 | 事件捕获与转发 | [03-event-capture.md](03-event-capture.md) | `monitor_hook.py`, `settings.json` |
| 4 | 服务端事件处理 | [04-event-processing.md](04-event-processing.md) | `event_server.rs`, `db.rs`, `state_machine.rs` |
| 5 | 状态机转换 | [05-state-machine.md](05-state-machine.md) | `state_machine.rs`, `lib.rs` |
| 6 | Session 生命周期 | [06-session-lifecycle.md](06-session-lifecycle.md) | `event_server.rs`, `state_machine.rs`, `db.rs`, `process_checker.rs` |
| 7 | 进程存活检测 | [07-process-checker.md](07-process-checker.md) | `process_checker.rs`, `monitor_hook.py` |
| 8 | 前端轮询与 UI 渲染 | [08-frontend-polling.md](08-frontend-polling.md) | `sessionStore.ts`, `FloatingPanel.vue`, `SessionCard.vue`, `ExpandedDetail.vue` |
| 9 | 窗口关闭与托盘 | [09-tray-close.md](09-tray-close.md) | `tray.rs`, `lib.rs`, `commands.rs` |
| 10 | 整体数据流总览 | [10-data-flow-overview.md](10-data-flow-overview.md) | 全部模块 |

## 快速导航

- **想了解事件如何从 CC 到 UI？** → 按顺序阅读: [3](03-event-capture.md) → [4](04-event-processing.md) → [5](05-state-machine.md) → [8](08-frontend-polling.md)
- **想了解 Session 如何创建/更新/销毁？** → [6](06-session-lifecycle.md) + [7](07-process-checker.md)
- **想了解应用启动做了什么？** → [1](01-app-startup.md)
- **想了解全部流程的端到端图？** → [10](10-data-flow-overview.md)

## 系统架构参考

```
┌─────────────────────────────────────────────────────────┐
│  桌面悬浮窗 (Vue 3 + Pinia)                              │
│  FloatingPanel → SessionCard + ExpandedDetail           │
│  sessionStore: 2s 轮询 get_sessions() IPC               │
├─────────────────────────────────────────────────────────┤
│  Tauri IPC Bridge (commands.rs)                         │
│  get_sessions / get_session_detail / get_session_events │
│  install_hooks_cmd / uninstall_hooks_cmd / ...          │
├─────────────────────────────────────────────────────────┤
│  Rust Backend (agentpulse_lib)                          │
│  ┌─────────────────┐  ┌────────────┐  ┌──────────────┐ │
│  │ event_server.rs  │  │ db.rs      │  │ hooks.rs     │ │
│  │ (tiny_http:17878)│  │ (SQLite)   │  │ (settings)   │ │
│  ├─────────────────┤  └────────────┘  └──────────────┘ │
│  │ state_machine.rs │  ┌──────────────────────────────┐ │
│  └─────────────────┘  │ process_checker.rs (sysinfo)  │ │
│                        └──────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│  Python Adapter (adapters/claude-code/)                  │
│  monitor_hook.py: stdin → HTTP POST :17878 + PID 探测   │
│  install_hooks.py: 管理 ~/.claude/settings.json hooks   │
├─────────────────────────────────────────────────────────┤
│  Claude Code CLI → triggers hooks → stdin → adapter     │
└─────────────────────────────────────────────────────────┘
```
