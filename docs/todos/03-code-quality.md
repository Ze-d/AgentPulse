# TODO: 代码质量改进

> 重构、去重、测试覆盖、工程规范

---

## 3.1 前端计算逻辑去重 🟡

**问题**: `SessionCard.vue` 和 `ExpandedDetail.vue` 各自独立计算 `statusColor`、`statusLabel`、`duration`，逻辑完全重复。

**建议**: 抽取为 composable `useSessionDisplay(session: AgentSession)`，或在 store 中添加 display getters。

---

## 3.2 Dead Code 清理 🟡

**问题**: 以下代码已实现但从未被调用：

| 代码 | 位置 |
|------|------|
| `selectedSessionId` | sessionStore.ts state |
| `selectedSession` getter | sessionStore.ts getter |
| `fetchSessionDetail` | sessionStore.ts action |
| `fetchSessionEvents` | sessionStore.ts action |
| `attentionSessions` getter | sessionStore.ts getter |
| `vue.svg` | assets 目录 |

**建议**: 清理 dead code 或实现对应 UI 功能（如 session 详情面板）。

---

## 3.3 DB 行映射代码去重 🟡

**问题**: `db.rs` 中 `list_all_sessions`、`get_session`、`list_sessions_with_pid` 三个方法有大量重复的列选择和行映射闭包（~60 行）。

**文件**: [apps/desktop/src-tauri/src/db.rs](../../apps/desktop/src-tauri/src/db.rs)

**建议**: 抽取 `fn map_session_row(row: &Row) -> Result<AgentSession>` 辅助函数。

---

## 3.4 冗余测试 🟢

**问题**: `db.rs` 模块内 `#[cfg(test)]` 的 8 个单元测试与 `tests/db_test.rs` 的 3 个集成测试覆盖相同路径。

**建议**: 移除模块内测试，统一使用集成测试。

---

## 3.5 前端无类型安全的 IPC 🟢

**问题**: `invoke<T>("command_name")` 对 command name 和返回值使用了运行时字符串匹配。命令名拼写错误只在运行时发现。

**建议**: 使用 `tauri-specta` 或手写一个 typed wrapper 层，将 `invoke("get_sessions")` 包装为 `getSessions(): Promise<AgentSession[]>`。

---

## 3.6 CSS 硬编码值抽取 🟢

**问题**: `FloatingPanel.vue` 的 `adjustWindowSize` 函数硬编码 `headerHeight = 28`、`cardHeight = 34`、`padding = 20`。与 CSS 中对应的值不同步。

**建议**: 使用 CSS 自定义属性并通过 `getComputedStyle` 读取，或至少定义为模块级常量。

---

## 3.7 Tailwind CSS 未使用 🟢

**问题**: `main.css` 通过 `@import "tailwindcss"` 加载了整个 Tailwind，但所有 `.vue` 模板均使用自定义 CSS，零个 utility class。

**建议**: 移除 Tailwind import 或开始在组件中使用 utility classes。

---

## 3.8 `App.vue` 与 `main.css` 样式重复 🟢

**问题**: `App.vue` 的 `<style>` 块重复了 `main.css` 中已定义的 `html, body, #app` reset 样式。

**建议**: 移除 App.vue 中的重复样式。

---

## 3.9 monitor_hook.py 重复函数 🟢

**问题**: `monitor_hook.py:140-148` 存在一个重复的 `read_stdin` 函数定义。

**文件**: [adapters/claude-code/monitor_hook.py](../../adapters/claude-code/monitor_hook.py)

**建议**: 移除重复定义。

---

## 3.10 console.debug 生产泄漏 🟢

**问题**: `sessionStore.ts:42` 在每次 poll 时输出 `console.debug("[AgentPulse] fetchSessions: ...")`，发布版不应有调试日志。

**建议**: 使用条件日志或 Tauri 的 log plugin。
