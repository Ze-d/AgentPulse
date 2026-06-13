# 15 — 项目健康检查：第三阶段 展示与可维护性

**状态：** 已完成 ✅
**优先级：** P2/P3 — 开源展示、遗留清理、长期可维护性
**创建日期：** 2026-06-13
**完成日期：** 2026-06-13
**来源：** 项目健康检查报告

---

## 概述

第三阶段聚焦于 **完善开源项目基础设施**、**清理遗留问题**、**提升新开发者体验**。这些改进直接影响项目在开源社区中的形象和长期可维护性。

---

## 15.1 🟡 P2 添加 Issue / PR Templates

**问题**：无 `.github/ISSUE_TEMPLATE/` 和 `.github/PULL_REQUEST_TEMPLATE.md`，社区反馈不规范。

**涉及文件**：
- 新建 `.github/ISSUE_TEMPLATE/bug_report.md`
- 新建 `.github/ISSUE_TEMPLATE/feature_request.md`
- 新建 `.github/PULL_REQUEST_TEMPLATE.md`

**操作**：

**Bug Report 模板内容**：
```markdown
---
name: Bug Report
about: 报告一个 bug
title: '[Bug]: '
labels: bug
---

### 描述
清晰描述 bug 现象。

### 复现步骤
1. 启动 AgentPulse ...
2. 执行 ...
3. 观察到 ...

### 期望行为
应发生什么。

### 实际行为
实际发生了什么（截图/日志）。

### 环境
- OS: [Windows 10/11, macOS 14, Ubuntu 24.04]
- AgentPulse 版本: [e.g. 0.4.0]
- Claude Code 版本: [e.g. 2.0.0]
```

**Feature Request 模板内容**：
```markdown
---
name: Feature Request
about: 建议新功能
title: '[Feat]: '
labels: enhancement
---

### 问题描述
这个功能解决什么问题？

### 建议方案
你期望的方案。

### 替代方案
你考虑过的其他方案。

### 额外上下文
截图、参考链接等。
```

**PR Template 内容**：
```markdown
### 变更描述
简述做了什么。

### 测试
- [ ] `cargo test` 通过
- [ ] `npm test` 通过
- [ ] `npx vue-tsc --noEmit` 通过
- [ ] 手动验证（如有）

### 关联 Issue
Closes #

### 截图（如涉及 UI 变更）
```

**验收标准**：
- 创建 issue 时可选择 Bug Report 或 Feature Request 模板
- PR 创建时自动填充 PR template

---

## 15.2 🟡 P2 创建社区文件

**问题**：缺少 CONTRIBUTING.md、CODE_OF_CONDUCT.md、SECURITY.md。

**涉及文件**：
- 新建 `CONTRIBUTING.md`
- 新建 `CODE_OF_CONDUCT.md`
- 新建 `SECURITY.md`

**操作**：

**CONTRIBUTING.md** 应包含：
- 开发环境搭建（引用 `docs/local-development-guide.md`）
- 分支策略和 commit 规范（引用 AGENTS.md 中的 Git Workflow）
- 运行测试的命令
- 运行 CI 检查的命令
- PR 提交流程

**SECURITY.md** 应包含：
- 安全漏洞报告渠道（GitHub Security Advisory 或邮箱）
- 响应时间承诺（如 48 小时内确认）
- 支持版本列表

**CODE_OF_CONDUCT.md**：
- 使用标准 Contributor Covenant v2.1

**验收标准**：
- 三个文件存在于仓库根目录
- CONTRIBUTING.md 中可执行命令均有效

---

## 15.3 🟢 P2 `.env.example` 完善

**问题**：当前 `.env.example` 只有一行注释 `# Copy to .env and fill in values`，未列出可用变量。

**涉及文件**：
- `.env.example`

**操作**：
```bash
# AgentPulse 环境变量配置
# 复制此文件为 .env 并修改值

# HTTP 事件服务器端口（默认 17878）
AGENTPULSE_PORT=17878

# 进程存活检查间隔（秒，默认 5）
AGENTPULSE_CHECK_INTERVAL=5

# 前端轮询间隔（毫秒，默认 2000）
AGENTPULSE_POLL_INTERVAL=2000

# 日志级别（默认 info，可选: trace, debug, info, warn, error）
RUST_LOG=info

# Hook 适配器日志级别（默认 info）
AGENTPULSE_LOG_LEVEL=info

# 事件服务器 URL（默认 http://127.0.0.1:17878/api/events）
# 通常不需要修改
# AGENTPULSE_URL=http://127.0.0.1:17878/api/events
```

**验收标准**：
- 每个环境变量有注释说明用途和默认值

---

## 15.4 🟢 P2 创建独立的 HTTP API 文档

**问题**：HTTP API（`/api/events`、`/api/sessions`、`/api/health`）仅在 README 中以 curl 示例隐式说明，没有结构化文档。

**涉及文件**：
- 新建 `docs/api.md`

**操作**：
创建 API 文档包含：

```markdown
# AgentPulse HTTP API

## 基础信息
- Base URL: `http://127.0.0.1:{port}`（默认 17878）
- Content-Type: `application/json`

## 端点

### POST /api/events
接收 hook 事件。

**Request Body**: (AgentEvent JSON)
**Response**: 201 + { event, session }

### GET /api/sessions
获取所有 session 列表。

**Response**: 200 + AgentSession[]

### GET /api/health
健康检查。

**Response**: 200 + {"status": "ok"}

## 错误码
| 状态码 | 含义 |
|--------|------|
| 201 | 事件已接收 |
| 400 | JSON 格式错误 |
| 404 | 路由不存在 |
| 500 | 服务器内部错误 |
```

**验收标准**：
- 第三方开发者可根据 api.md 独立接入 AgentPulse 事件系统

---

## 15.5 🟢 P3 创建 `rust-toolchain.toml`

**问题**：CI 用 `dtolnay/rust-toolchain@stable`，本地无固定 toolchain 版本。不同开发者可能用不同 Rust 版本导致 CI 与本地行为不一致。

**涉及文件**：
- 新建 `apps/desktop/src-tauri/rust-toolchain.toml`

**操作**：
```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

**验收标准**：
- `rustup show` 在 `src-tauri/` 目录下显示 pinned toolchain
- 不影响 hook-adapter 的构建

---

## 15.6 🟢 P3 提取 `lib.rs` setup 闭包中的重复 hook 安装逻辑

**问题**：[lib.rs:258-338](../../apps/desktop/src-tauri/src/lib.rs#L258-L338) 中 CC hook 和 Codex hook 的安装线程代码几乎相同（resource_dir 获取 → hook binary 提取 → ensure 安装 → tracing 日志），约 40 行重复。

**涉及文件**：
- `apps/desktop/src-tauri/src/lib.rs`

**操作**：
1. 提取公共逻辑为函数：
```rust
fn auto_install_hooks(
    app_handle: &tauri::AppHandle,
    hook_type: &str, // "Claude Code" or "Codex"
    settings_key: &str,
) {
    // 公共逻辑
}
```
2. 在 setup 闭包中用两个 `std::thread::spawn` 调用

**验收标准**：
- 行为不变
- `cargo clippy -- -D warnings` 零警告
- `lib.rs` 行数显著减少

---

## 15.7 🟢 P3 清理 `docs/superpowers/` 中已完成的设计文档

**问题**：`docs/superpowers/` 下积累了大量设计和计划文档（specs + plans），多数任务已完成。应评估是否需要保留或归档。

**涉及文件**：
- `docs/superpowers/specs/` — 6 个设计文档
- `docs/superpowers/plans/` — 5 个计划文档

**操作**：
1. 检查每个 spec/plan 对应的任务是否已完成
2. 已完成的移到 `docs/superpowers/archive/` 或删除
3. 保留仍在规划中的文档

**验收标准**：
- `docs/superpowers/` 目录只包含当前活跃的文档
- 历史文档可追溯（归档而非删除）

---

## 15.8 🟢 P3 考虑提升版本号

**问题**：当前版本 0.4.0，但项目已实现：配置系统、tracing 日志、多源支持（CC+Codex）、进程监控、系统托盘、状态机——功能成熟度远超典型 0.x 项目。低版本号可能让潜在用户低估项目成熟度。

**涉及文件**：
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/package.json`
- `adapters/hook-adapter/Cargo.toml`
- `apps/desktop/src-tauri/tauri.conf.json`

**操作**：
- 完成 Phase 1 + Phase 2 主要修复后，评估是否 bump 到 0.5.0 或 1.0.0-beta

**注意事项**：
- 版本号决策需项目负责人确认
- 不影响任何技术行为

---

## 关联

- [[13-health-check-phase1-critical-fixes]] — 第一阶段
- [[14-health-check-phase2-quality]] — 第二阶段
- [[08-infra-improvements]] — 8.6 缺少 rust-toolchain.toml
- [[09-git-history-cleanup]] — Git 历史清理
