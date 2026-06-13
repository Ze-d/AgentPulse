# 13 — 项目健康检查：第一阶段 关键修复

**状态：** 部分完成（13.1 ✓, 13.2 ✓, 13.3 ⊘ 跳过, 13.4 ⊘ 跳过, 13.5 ✓, 13.6 ✓）
**优先级：** P0/P1 — 阻塞新用户体验和 Release 稳定性
**创建日期：** 2026-06-13
**完成日期：** 2026-06-13
**来源：** 项目健康检查报告

---

## 概述

第一阶段聚焦于 **消除文档与代码不一致** 和 **修复阻塞性问题**。这些问题直接影响新用户能否成功运行项目，以及 Release 构建是否稳定。

---

## 13.1 🔴 P0 文档全面过期：Python → Rust 迁移后未更新

**问题**：Hook 适配器已从 Python 迁移到 Rust（`adapters/hook-adapter/`），但以下文档仍引用已删除的 Python 文件和 Python 依赖：

| 文档 | 过时内容 |
|------|----------|
| `README.md` L57-62 | 环境要求表列出 Python >= 3.8 — 现在不需要 |
| `README.md` L126 | 配置表包含 `python` 字段说明 — 已从 config.rs 移除 |
| `README.md` L157 | 技术栈表 "适配器: Python 3 标准库, 无第三方依赖" |
| `README.md` L189 | 项目结构列出 `adapters/claude-code/agentpulse-hook` — 已移至 `adapters/hook-adapter/` |
| `AGENTS.md` L112-113 | Key Files 表指向不存在的 `adapters/claude-code/install_hooks.py` 和 `monitor_hook.py` |
| `docs/local-development-guide.md` L52-53 | 项目结构图列出 Python 文件 |
| `docs/local-development-guide.md` L154-158 | "测试 Python Hook 适配器" 部分命令过时 |
| `docs/testing/testing-strategy.md` L9 | "Python unit: install_hooks.py (22 cases)" — 不存在 |
| `docs/architecture/overview.md` L17 | 技术栈表 "Adapter: Python 3" |
| `docs/architecture/overview.md` L49 | 数据流提到 `monitor_hook.py` |

**涉及文件**：
- `README.md`
- `AGENTS.md`
- `docs/local-development-guide.md`
- `docs/testing/testing-strategy.md`
- `docs/architecture/overview.md`

**操作**：
1. 更新 `README.md`：移除 Python 环境要求、更新技术栈表、更新配置表、更新项目结构
2. 更新 `AGENTS.md`：Key Files 表指向 Rust 文件
3. 更新 `docs/local-development-guide.md`：项目结构图、测试命令
4. 更新 `docs/testing/testing-strategy.md`：移除 Python 测试行
5. 更新 `docs/architecture/overview.md`：技术栈和数据流描述

**验收标准**：
- `grep -r "monitor_hook\|install_hooks\.py\|Python 3.*适配" docs/ README.md AGENTS.md` 返回零结果（除非引用历史 CHANGELOG）
- 新用户按 README 步骤能正确理解项目依赖

---

## 13.2 🔴 P1 根 `package.json` 依赖残留

**问题**：根目录 `package.json` 包含 `vitest@^4.1.7`、`@vue/test-utils@^2.4.10`、`happy-dom@^20.9.0`，与 `apps/desktop/package.json` 版本不一致，且根目录无 scripts 使用它们。

**涉及文件**：
- `package.json`（根目录）

**操作**：
1. 清理根 `package.json` 的 devDependencies（或删除整个文件如果确认无用途）

**验收标准**：
- 版本冲突消除
- `npm install` 在根目录行为明确

---

## 13.3 🔴 P1 `tauri-action@v0` 用于 Release 构建

**问题**：`.github/workflows/release.yml` L139 使用 `tauri-apps/tauri-action@v0`，该版本已废弃。最新稳定版是 `v2`。

**涉及文件**：
- `.github/workflows/release.yml`

**操作**：
```diff
- uses: tauri-apps/tauri-action@v0
+ uses: tauri-apps/tauri-action@v2
```
对照 [tauri-action v2 文档](https://github.com/tauri-apps/tauri-action) 验证参数兼容性（`projectPath`、`tagName`、`releaseName`、`releaseBody`、`args`）。

**验收标准**：
- 下次 tag push 能成功触发 release 构建

---

## 13.4 🔴 P1 内存数据库在重启时丢失数据

**问题**：[lib.rs:138](../../apps/desktop/src-tauri/src/lib.rs#L138) 使用 `Database::new_in_memory()`，README 声称 "SQLite 持久化存储" 但实际数据不持久化。文件持久化方法 `Database::new(path)` 已实现但未使用。

**涉及文件**：
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src-tauri/src/config.rs`（可能需要加配置项）

**操作**：
1. 在 `AgentPulseConfig` 中添加 `db_path` 字段（可选，null = 默认路径）
2. 将 `lib.rs` 中 `Database::new_in_memory()` 改为 `Database::new(&app_data_dir.join("sessions.db"))`
3. 或添加配置开关 `persist_sessions: bool`

**验收标准**：
- AgentPulse 重启后 session 历史不丢失
- 配置可控制持久化行为
- README 描述与实际行为一致

---

## 13.5 🔴 P1 `normalize_codex_event` 与 `normalize_claude_code_event` 代码重复

**问题**：[event_server.rs:20-75](../../apps/desktop/src-tauri/src/event_server.rs#L20-L75) 和 [event_server.rs:84-133](../../apps/desktop/src-tauri/src/event_server.rs#L84-L133) 约 80% 代码重复。公共逻辑包括：字段提取（hook_event_name, session_id, cwd 等）、project_name 推导、AgentEvent 构造。

**涉及文件**：
- `apps/desktop/src-tauri/src/event_server.rs`

**操作**：
1. 提取公共字段提取逻辑为内部函数 `extract_common_fields(raw: &Value) -> CommonFields`
2. 差异部分（hook_event_name → EventType/AgentStatus 映射表）作为策略参数
3. 保留两个公开函数但内部委托给通用实现

```rust
// 策略表
type EventMapping = fn(&str, &str) -> (EventType, AgentStatus);

fn normalize_event_inner(raw: &Value, source: AgentSource, mapping: EventMapping) -> AgentEvent {
    // 公共逻辑
}
```

**验收标准**：
- 所有 17 个 event_server 测试通过
- `cargo clippy -- -D warnings` 零警告
- 公共逻辑不重复

---

## 13.6 🔴 P1 前端 UI 组件测试严重不足

**问题**：核心 UI 组件缺少测试：

| 组件 | 现有测试 |
|------|----------|
| `SessionCard.vue` | 1 个 |
| `FloatingPanel.vue` | 0 个 |
| `ExpandedDetail.vue` | 0 个 |
| `useSwipeDismiss` | 0 个 |

**涉及文件**：
- `apps/desktop/src/components/` — 需要新增 `__tests__/` 目录
- `apps/desktop/src/composables/` — 需要新增 `__tests__/` 目录

**操作**：
1. **SessionCard 测试**：验证 status color 渲染、swipe gesture 触发、click event emit
2. **FloatingPanel 测试**：验证 empty state 显示、session list 渲染、error banner 显示/关闭
3. **ExpandedDetail 测试**：验证 detail fields 渲染、openDir/openTranscript emit
4. **useSwipeDismiss 测试**：验证 touch gesture 解析、threshold 检测、dismiss callback 触发、mouse 坐标跟踪

**验收标准**：
- `npm test` 全部通过
- 每个组件至少覆盖 normal path + error/edge case

---

## 关联

- [[13-health-check-phase2-quality]] — 第二阶段
- [[13-health-check-phase3-polish]] — 第三阶段
- [[03-code-quality]] — 3.5 IPC 类型安全（未完成）
- [[08-infra-improvements]] — tiny_http 替换等技术债
