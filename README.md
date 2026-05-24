# AgentPulse

本地 AI Coding Agent 运行状态监控器，通过桌面悬浮窗统一展示 Claude Code 等 CLI coding agent 的运行状态。

## 功能

- **实时监控** — 通过 Claude Code hooks 捕获 session 生命周期事件（SessionStart、PreToolUse、PostToolUse、Notification、Stop 等）
- **桌面悬浮窗** — Tauri 2 无边框置顶窗口，显示活跃 session 卡片（项目名、状态、工具、持续时间）
- **状态机** — 规范化 agent 状态流转（Starting → Running → ToolRunning → WaitingInput → Completed）
- **本地优先** — SQLite 持久化，不上传源码或对话记录

## 快速开始

### 环境要求

- Node.js >= 18
- Rust >= 1.70 (MSVC toolchain on Windows)
- Python >= 3.8

### 安装运行

```powershell
# 安装前端依赖
cd apps/desktop
npm install

# 启动 Tauri 开发模式
npm run tauri dev

# 安装 Claude Code hooks（新终端）
python adapters/claude-code/install_hooks.py

# 验证 hooks 状态
python adapters/claude-code/install_hooks.py --status
```

启动后正常使用 Claude Code，AgentPulse 浮窗自动显示 session 状态。

### 运行测试

```powershell
# Python 单元测试
python -m pytest tests/unit/ -v

# Python E2E 测试（需 AgentPulse 运行中）
python tests/integration/test_e2e.py

# Rust 测试
cd apps/desktop/src-tauri
cargo test

# TypeScript 类型检查
cd apps/desktop
npx vue-tsc --noEmit
```

## 架构

```
Claude Code session 事件
  → ~/.claude/settings.json (hooks 配置)
    → monitor_hook.py (stdin 适配器)
      → POST /api/events (127.0.0.1:17878)
        → event_server.rs (规范化 + 状态机)
          → SQLite (持久化)
            → Tauri commands (IPC)
              → Vue 3 前端 (轮询展示)
```

| 层 | 技术 |
|---|------|
| 桌面壳 | Tauri 2 |
| 前端 | Vue 3 + TypeScript + Tailwind CSS |
| 后端 | Rust (tiny_http, rusqlite, serde) |
| 数据库 | SQLite (rusqlite bundled) |
| 适配器 | Python 3 (install_hooks.py, monitor_hook.py) |

## 项目结构

```
AgentPulse/
├── apps/desktop/                  # Tauri 桌面应用
│   ├── src/                       # Vue 3 前端
│   │   ├── components/            # FloatingPanel, SessionCard, ExpandedDetail
│   │   ├── stores/                # Pinia sessionStore (2s 轮询)
│   │   └── types/                 # TypeScript 类型定义
│   ├── src-tauri/                 # Rust 后端
│   │   ├── src/
│   │   │   ├── lib.rs             # 共享类型 + run() 入口
│   │   │   ├── db.rs              # SQLite CRUD
│   │   │   ├── state_machine.rs   # 状态转换 + needs_attention
│   │   │   ├── event_server.rs    # HTTP 服务器 :17878
│   │   │   ├── commands.rs        # Tauri IPC 命令
│   │   │   ├── tray.rs            # 系统托盘
│   │   │   └── main.rs            # 二进制入口
│   │   └── tests/                 # Rust 集成测试
│   └── tauri.conf.json
├── adapters/claude-code/          # Claude Code hook 适配器
│   ├── install_hooks.py           # 一键安装/卸载/状态/预览
│   └── monitor_hook.py            # stdin → HTTP 转发 + 重试
├── tests/
│   ├── unit/                      # Python 单元测试 (30 个)
│   └── integration/               # E2E 冒烟测试
└── docs/                          # 文档
    ├── architecture/              # 架构设计
    ├── testing/                   # 测试策略 + TDD 指南
    ├── ai/                        # AI 协作规范
    ├── superpowers/               # 设计文档 + 实现计划
    ├── fixlog/                    # Bug 修复记录
    └── todos/                     # 待办事项
```

## 文档索引

- [本地开发指南](docs/local-development-guide.md)
- [架构概述](docs/architecture/overview.md)
- [测试策略](docs/testing/testing-strategy.md)
- [TDD 指南](docs/testing/tdd-guide.md)
- [代码规范](docs/ai/coding-rules.md)
- [Context Map](docs/ai/context-map.md)
- [Bug 修复记录](docs/fixlog/)
