# AgentPulse v0.1 设计文档

**日期：** 2026-05-22
**状态：** 已批准

## 概述

AgentPulse 是一款面向 AI 编程代理（Agent）的本地桌面监视器。v0.1 聚焦于 Claude Code 集成：通过 hooks 捕获生命周期事件，持久化到 SQLite，并在无边框悬浮窗中实时展示状态。

**产品定位：** 本地优先。无云同步。不上传源代码。不上传对话记录。

## 技术栈

| 层次 | 技术 | 职责 |
|-------|-----------|------|
| 桌面外壳 | Tauri 2 | 窗口管理、托盘、Rust 运行时 |
| 前端 | Vue 3 + TypeScript + Tailwind CSS | 悬浮窗 UI |
| 后端 | Rust（Tauri 核心） | HTTP 服务、状态机、SQLite、进程监视 |
| 数据库 | SQLite（通过 rusqlite） | 事件、会话、设置 |
| 通信 | Tauri 事件（invoke/emit） | Rust ↔ Vue 数据流 |

## 架构：四层结构

```
┌─────────────────────────────────┐
│  悬浮窗（Vue3 + Tailwind）        │  ← Tauri 事件桥接
├─────────────────────────────────┤
│  本地监视核心（Rust）              │  ← HTTP :17878 + SQLite + 状态机
├─────────────────────────────────┤
│  Hook 输入  │  文件输入  │ 进程输入  │ ← v0.1：仅 Hook 输入
└─────────────────────────────────┘
```

## v0.1 范围

### 包含内容
- Tauri 2 无边框悬浮窗（始终置顶、可拖拽）
- 系统托盘（显示/隐藏、退出）
- Rust HTTP 服务，监听 127.0.0.1:17878
- SQLite 持久化：事件、会话
- 通过轻量适配脚本集成 Claude Code hooks
- 状态卡片：Agent 来源、项目名、状态、持续时间、最近工具
- 卡片展开/收起查看详情
- 会话状态机

### 不包含（延期）
- Codex 集成 → v0.2
- 对话记录文件解析 → v0.2
- 进程树扫描 → v0.3
- 历史页面 → v0.2
- WSL 支持 → v0.3
- 多项目并行监视 → v0.3
- 插件架构 → v1.0

## 统一事件模型

```typescript
type AgentSource = "claude-code";

type AgentStatus =
  | "starting"
  | "running"
  | "tool_running"
  | "waiting_input"
  | "waiting_permission"
  | "completed"
  | "failed"
  | "unknown";

interface AgentEvent {
  id: string;           // UUID v4，由收集器生成
  source: AgentSource;
  sessionId: string;    // 来自 CC hook 的 session_id
  cwd: string;
  projectName?: string; // 由 cwd 的目录名派生
  eventType:
    | "session_start"
    | "pre_tool_use"
    | "post_tool_use"
    | "permission_request"
    | "notification"
    | "stop"
    | "failure";
  status: AgentStatus;
  message?: string;
  toolName?: string;
  transcriptPath?: string;
  createdAt: number;    // Unix 毫秒时间戳
}

interface AgentSession {
  sessionId: string;
  source: AgentSource;
  cwd: string;
  projectName: string;
  status: AgentStatus;
  startedAt: number;
  updatedAt: number;
  completedAt?: number;
  lastMessage?: string;
  lastToolName?: string;
  transcriptPath?: string;
  needsAttention: boolean;
}
```

## Claude Code Hook 事件映射（v0.1）

| Hook 事件 | AgentPulse 状态 | 备注 |
|-----------|------------------|-------|
| SessionStart | starting → running | 创建会话记录 |
| PreToolUse | tool_running | 捕获 tool_name |
| PostToolUse | running | 捕获 tool_response 摘要 |
| PostToolUseFailure | failed | 捕获错误信息 |
| Notification（permission_prompt） | waiting_permission | needsAttention = true |
| Notification（idle_prompt） | waiting_input | needsAttention = true |
| Stop | completed | 记录 completedAt |

## 状态机

```
unknown → SessionStart → starting →（首次事件）→ running
running → PreToolUse → tool_running
tool_running → PostToolUse → running
running → Notification（permission_prompt）→ waiting_permission
running → Notification（idle_prompt）→ waiting_input
waiting_permission →（任意工具事件）→ tool_running
waiting_input →（任意工具事件）→ tool_running
running → Stop → completed
tool_running → PostToolUseFailure → failed
```

`needsAttention` 为 true 当状态属于 {waiting_input, waiting_permission, completed, failed}。

## Rust 后端模块

| 模块 | 职责 |
|--------|---------------|
| `event_server` | HTTP 服务，监听 127.0.0.1:17878，接受 POST /api/events |
| `state_machine` | 状态转换逻辑、状态验证 |
| `db` | 事件和会话的 SQLite CRUD 操作 |
| `tray` | 系统托盘图标和菜单 |
| `window` | 悬浮窗创建和管理 |
| `commands` | Tauri #[command] 处理器，供前端 invoke 调用 |

## HTTP API（内部）

```
POST /api/events
  Body: AgentEvent（JSON）
  Response: 201 { id, sessionId }

GET /api/sessions
  Response: [AgentSession]

GET /api/sessions/:id
  Response: AgentSession

GET /api/health
  Response: 200 { status: "ok" }
```

## Hook 适配器

Claude Code hooks 配置在 `~/.claude/settings.json` 或项目级的 `.claude/settings.json` 中。每个 hook 事件触发一个轻量适配脚本（Python），该脚本：

1. 从 stdin/argv 读取 hook JSON
2. 规范化为 `AgentEvent` 格式
3. POST 到 `http://127.0.0.1:17878/api/events`

## 悬浮窗 UI

### 默认（紧凑模式）— 320×200px
- 始终置顶、无边框、可拖拽
- Catppuccin Mocha 深色主题
- 会话卡片，左侧颜色条（运行中=绿色、等待中=橙色、已完成=蓝色、失败=红色）
- 每张卡片：Agent 图标、项目名、状态指示器、持续时间、最近工具名
- needsAttention 的会话：脉冲闪烁提示

### 展开（点击卡片）— 320×280px
- 完整详情：状态、工具、事件、工作目录、持续时间、PID
- 最近消息摘要
- 操作链接：打开项目目录、打开对话记录

### 托盘菜单
- 显示/隐藏悬浮窗
- 暂停监视
- 退出

## 数据库设计（SQLite）

```sql
CREATE TABLE sessions (
  session_id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  cwd TEXT NOT NULL,
  project_name TEXT,
  status TEXT NOT NULL DEFAULT 'unknown',
  started_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  completed_at INTEGER,
  last_message TEXT,
  last_tool_name TEXT,
  transcript_path TEXT,
  needs_attention INTEGER DEFAULT 0
);

CREATE TABLE events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  source TEXT NOT NULL,
  event_type TEXT NOT NULL,
  status TEXT NOT NULL,
  cwd TEXT,
  message TEXT,
  tool_name TEXT,
  transcript_path TEXT,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(session_id)
);

CREATE INDEX idx_events_session ON events(session_id);
CREATE INDEX idx_events_created ON events(created_at);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_updated ON sessions(updated_at);
```

## 项目结构

```
AgentPulse/
├── apps/desktop/              # Tauri 应用（由 create-tauri-app 创建）
│   ├── src/                   # Vue 3 前端
│   │   ├── components/
│   │   │   ├── FloatingPanel.vue
│   │   │   ├── SessionCard.vue
│   │   │   ├── ExpandedDetail.vue
│   │   │   └── TrayMenu.vue
│   │   ├── stores/
│   │   │   └── sessionStore.ts
│   │   ├── types/
│   │   │   └── agent.ts
│   │   ├── App.vue
│   │   └── main.ts
│   ├── src-tauri/
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   ├── event_server.rs
│   │   │   ├── state_machine.rs
│   │   │   ├── db.rs
│   │   │   ├── tray.rs
│   │   │   ├── window.rs
│   │   │   └── commands.rs
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── tailwind.config.js
├── adapters/
│   └── claude-code/
│       ├── monitor_hook.py
│       └── install_hooks.py
├── tests/
│   ├── unit/
│   ├── integration/
│   └── fixtures/
└── docs/
    └── superpowers/specs/
        └── 2026-05-22-agentpulse-v01-design.md
```

## 测试策略

| 层次 | 工具 | 范围 |
|-------|------|-------|
| Rust 单元测试 | `#[cfg(test)]` + cargo test | state_machine、db、event_server 处理器 |
| Rust 集成测试 | cargo test（integration） | HTTP 端点 + 数据库完整链路 |
| Vue 单元测试 | Vitest + vue-test-utils | 组件渲染、store 逻辑 |
| Vue e2e | Tauri + Playwright（延期至 v0.2） | 完整窗口流程 |
| 适配器 | pytest（延期） | Hook JSON 规范化 |

## 关键设计决策

1. **不用 Electron** — Tauri 更轻量，Rust 更适合系统级监控
2. **用 HTTP，不用 stdin** — hook 适配器 POST 到本地服务，而非直接耦合 Tauri 进程；解耦 hook 生命周期与 app 生命周期
3. **用 SQLite，不用 JSON 文件** — 可查询历史记录、HTTP 处理器场景下并发写入安全
4. **Hook 优先，不做终端抓取** — Claude Code hooks 是权威事件源；终端输出不可靠
5. **单窗口、多卡片** — 一个悬浮窗以堆叠卡片形式展示所有活跃会话；比每个会话一个窗口更简单易管理
