# Module Boundaries

## Modules

| Module | Responsibility | Dependencies |
|--------|---------------|--------------|
| `adapters/claude-code/install_hooks.py` | 安装/卸载/状态/预览 hooks 到 ~/.claude/settings.json | Python stdlib (json, argparse, shutil, pathlib) |
| `adapters/claude-code/monitor_hook.py` | 从 stdin 读取 hook JSON，POST 到事件服务器（含重试 + PID 探测） | Python stdlib (json, urllib, logging, argparse) |
| `apps/desktop/src-tauri/src/lib.rs` | 共享 Rust 类型 (AgentEvent, AgentSession, AgentStatus, EventType, AgentSource) + run() 入口 | serde, serde_json |
| `apps/desktop/src-tauri/src/config.rs` | 配置加载：config.json + 环境变量覆盖，首次启动生成默认配置 | serde, serde_json |
| `apps/desktop/src-tauri/src/logging.rs` | tracing 初始化：stderr 文本 + JSON 文件轮转，7 天日志清理 | tracing, tracing-subscriber, tracing-appender |
| `apps/desktop/src-tauri/src/db.rs` | SQLite 数据库：schema init, CRUD for sessions + events, enum serialize/deserialize, 定期清理 | rusqlite, serde_json |
| `apps/desktop/src-tauri/src/state_machine.rs` | 状态转换验证 (transition) + 终态恢复 + needs_attention 判断 | lib.rs (AgentStatus, EventType) |
| `apps/desktop/src-tauri/src/process_checker.rs` | 进程存活检测：可配置间隔轮询，跳过终态 session，删除僵尸活跃 session | lib.rs (AgentStatus), db, sysinfo |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP 服务器 :port：POST /api/events, GET /api/sessions, GET /api/health，完整错误日志 + 优雅关闭 | tiny_http, serde_json, db, state_machine, lib |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri IPC 命令：get_sessions, get_session_detail, get_session_events, delete_session, get_config + hook/tray 管理 | tauri, db, lib, config |
| `apps/desktop/src-tauri/src/hooks.rs` | Hook 配置管理：build, ensure, unregister, get_status + Python 解释器解析 | serde_json, std::process |
| `apps/desktop/src-tauri/src/tray.rs` | 系统托盘：show/hide/quit 菜单，编译时嵌入托盘图标 (`include_bytes!`) | tauri (tray-icon, image-ico feature) |
| `apps/desktop/src/types/agent.ts` | TypeScript 类型 (AgentEvent, AgentSession, AgentStatus, AgentSource...) + STATUS_LABELS, STATUS_COLORS, formatDuration | none |
| `apps/desktop/src/composables/useSessionDisplay.ts` | 共享 composable：statusColor, statusLabel, duration 计算 (接收 Ref<AgentSession>) | vue, types/agent |
| `apps/desktop/src/composables/useSwipeDismiss.ts` | 滑动关闭 composable：touch + mouse 滑动检测，80px 阈值，回弹/消失动画 | vue |
| `apps/desktop/src/utils/sourceDisplay.ts` | Agent source 缩写映射 (cc/cx/gm/cp) | types/agent |
| `apps/desktop/src/utils/openActions.ts` | openDirectory / openTranscript：openPath 封装 + 错误抛出 | @tauri-apps/plugin-opener |
| `apps/desktop/src/utils/ipc.ts` | 类型化 IPC 封装：getSessions, getConfig, hideMainWindow, deleteSession | @tauri-apps/api/core (invoke) |
| `apps/desktop/src/utils/logger.ts` | 前端日志工具：条件日志 + Tauri log_event 转发 | @tauri-apps/api/core |
| `apps/desktop/src/stores/sessionStore.ts` | Pinia store：sessions, isLoading, polling, expand/collapse, dismissSession, clearError | @tauri-apps/api/core (invoke), types/agent |
| `apps/desktop/src/components/FloatingPanel.vue` | 主面板：标题栏（刷新+最小化）、可关闭错误横幅、加载/空状态、slide 过渡 session 列表 | sessionStore, SessionCard, ExpandedDetail, openActions, ipc |
| `apps/desktop/src/components/SessionCard.vue` | 单 session 卡片：source缩写、项目名 tooltip、持续时间、状态、工具名、needsAttention 脉冲动画。completed 状态卡片支持 swipe-to-dismiss | types/agent, sourceDisplay, useSessionDisplay, useSwipeDismiss |
| `apps/desktop/src/components/ExpandedDetail.vue` | 展开详情：完整信息 + open dir / open transcript 操作按钮 | types/agent, useSessionDisplay |
| `tests/unit/test_install_hooks.py` | install_hooks.py 单元测试 (22 个) | pytest, install_hooks |
| `tests/unit/test_monitor_hook.py` | monitor_hook.py 单元测试 (10 个) | pytest, unittest.mock, monitor_hook |
| `tests/integration/test_e2e.py` | E2E 冒烟测试：POST 5 事件 → 验证 session 状态 | Python stdlib (urllib, json) |

## Dependency Graph

```
config.rs ──→ (独立，仅在 lib.rs 中加载)
logging.rs ──→ (独立，仅在 lib.rs 中初始化)

event_server.rs ──→ db.rs ──→ lib.rs
     │                ↑
commands.rs ──────────┘ ←── config.rs
     │
tray.rs

process_checker.rs ──→ db.rs ──→ lib.rs

monitor_hook.py ──HTTP POST──→ event_server.rs
install_hooks.py ──writes──→ ~/.claude/settings.json

FloatingPanel.vue ──→ sessionStore.ts ──invoke──→ commands.rs
        │               │        │
        │               │   ipc.ts (getConfig, getSessions, deleteSession)
        │               │   openActions.ts ──→ @tauri-apps/plugin-opener
        │               │
SessionCard.vue    ExpandedDetail.vue
        │               │
 sourceDisplay.ts   useSessionDisplay.ts
 useSwipeDismiss.ts
```

## Boundary Rules

- **Python ↔ Rust**: 仅通过 HTTP (POST /api/events)，无直接导入
- **Rust ↔ Vue**: 仅通过 Tauri IPC (invoke), 无直接 DOM 操作
- **Config ↔ All**: config.rs 在 lib.rs 加载后分发到各模块（参数传递 + AppState）
- **测试 ↔ 源码**: 仅通过公开 API，不访问内部实现
- **adapters/ 独立于 apps/**: 可单独部署和测试，不依赖 Tauri 运行时
