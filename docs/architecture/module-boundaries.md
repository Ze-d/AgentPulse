# Module Boundaries

## Modules

| Module | Responsibility | Dependencies |
|--------|---------------|--------------|
| `adapters/claude-code/install_hooks.py` | 安装/卸载/状态/预览 hooks 到 ~/.claude/settings.json | Python stdlib (json, argparse, shutil, pathlib) |
| `adapters/claude-code/monitor_hook.py` | 从 stdin 读取 hook JSON，POST 到事件服务器（含重试） | Python stdlib (json, urllib, logging, argparse) |
| `apps/desktop/src-tauri/src/lib.rs` | 共享 Rust 类型 (AgentEvent, AgentSession, AgentStatus, EventType, AgentSource) + run() 入口 | serde, serde_json |
| `apps/desktop/src-tauri/src/db.rs` | SQLite 数据库：schema init, CRUD for sessions + events, enum serialize/deserialize | rusqlite, serde_json |
| `apps/desktop/src-tauri/src/state_machine.rs` | 状态转换验证 (transition) + needs_attention 判断 | lib.rs (AgentStatus, EventType) |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP 服务器 :17878：POST /api/events, GET /api/sessions, GET /api/health | tiny_http, serde_json, db, state_machine, lib |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri IPC 命令：get_sessions, get_session_detail, get_session_events | tauri, db, lib |
| `apps/desktop/src-tauri/src/tray.rs` | 系统托盘：show/hide/quit 菜单 | tauri (tray-icon feature) |
| `apps/desktop/src/types/agent.ts` | TypeScript 类型 (AgentEvent, AgentSession, AgentStatus...) + STATUS_LABELS, STATUS_COLORS, formatDuration | none |
| `apps/desktop/src/stores/sessionStore.ts` | Pinia store：sessions, polling, expand/collapse, error handling | @tauri-apps/api/core (invoke), types/agent |
| `apps/desktop/src/components/FloatingPanel.vue` | 主面板：标题栏、错误横幅、空状态、session 列表 | sessionStore, SessionCard, ExpandedDetail |
| `apps/desktop/src/components/SessionCard.vue` | 单 session 卡片：状态圆点、项目名、持续时间、工具名 | types/agent |
| `apps/desktop/src/components/ExpandedDetail.vue` | 展开详情：完整信息 + 操作按钮 | types/agent, @tauri-apps/plugin-shell |
| `tests/unit/test_install_hooks.py` | install_hooks.py 单元测试 (22 个) | pytest, install_hooks |
| `tests/unit/test_monitor_hook.py` | monitor_hook.py 单元测试 (8 个) | pytest, unittest.mock, monitor_hook |
| `tests/integration/test_e2e.py` | E2E 冒烟测试：POST 5 事件 → 验证 session 状态 | Python stdlib (urllib, json) |

## Dependency Graph

```
event_server.rs ──→ db.rs ──→ lib.rs
     │                ↑
commands.rs ──────────┘
     │
tray.rs

monitor_hook.py ──HTTP POST──→ event_server.rs
install_hooks.py ──writes──→ ~/.claude/settings.json

FloatingPanel.vue ──→ sessionStore.ts ──invoke──→ commands.rs
        │                    │
SessionCard.vue    ExpandedDetail.vue
```

## Boundary Rules

- **Python ↔ Rust**: 仅通过 HTTP (POST /api/events)，无直接导入
- **Rust ↔ Vue**: 仅通过 Tauri IPC (invoke), 无直接 DOM 操作
- **测试 ↔ 源码**: 仅通过公开 API，不访问内部实现
- **adapters/ 独立于 apps/**: 可单独部署和测试，不依赖 Tauri 运行时
