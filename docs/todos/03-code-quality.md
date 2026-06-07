# ~~TODO: 代码质量改进~~ (8/10 完成)

> 状态：**8/10 已完成** — 3.4, 3.5 未完成（验证日期: 2026-06-07）

---

## 3.1 前端计算逻辑去重 ✅

**已完成**: 创建 `composables/useSessionDisplay.ts`，`SessionCard.vue` 和 `ExpandedDetail.vue` 均使用该 composable。

---

## 3.2 Dead Code 清理 ✅

**已完成**: `selectedSessionId`、`selectedSession`、`fetchSessionDetail`、`fetchSessionEvents` 已从 sessionStore 移除。`attentionSessions`/`expandedSession` getter 仍在用。`vue.svg` 已移除。

---

## 3.3 DB 行映射代码去重 ✅

**已完成**: `db.rs` L120-138 抽取 `map_session_row` 辅助函数，`list_all_sessions`、`get_session`、`list_sessions_with_pid` 统一使用。

---

## 3.4 冗余测试 🔴 未完成

**问题**: `db.rs` 模块内 `#[cfg(test)]` 的 14 个单元测试仍然保留。原计划的集成测试 `tests/db_test.rs` 不存在。模块内测试仍是唯一的测试覆盖。

**建议**: 要么移除模块内测试并创建集成测试，要么直接关闭此任务。

---

## 3.5 前端无类型安全的 IPC 🔴 未完成

**问题**: `ipc.ts` 仍然使用 `invoke<T>("command_name")` 字符串匹配。命令名拼写错误仅在运行时发现。

**建议**: 使用 `tauri-specta` 或手写完整的 typed wrapper 层。

---

## 3.6 CSS 硬编码值抽取 ✅

**已完成**: `FloatingPanel.vue` L13-15 将 `HEADER_HEIGHT = 28`、`CARD_HEIGHT = 34`、`PANEL_PADDING = 20` 定义为模块级常量。

---

## 3.7 Tailwind CSS 未使用 ✅

**已完成**: `main.css` 已移除 `@import "tailwindcss"`，仅保留 `:root` 变量和基础 reset。

---

## 3.8 `App.vue` 与 `main.css` 样式重复 ✅

**已完成**: `App.vue` 的 `<style>` 块已移除，样式统一在 `main.css` 管理。

---

## 3.9 monitor_hook.py 重复函数 ✅

**已完成**: `monitor_hook.py` 仅有一个 `read_stdin` 函数定义（L56-66），重复已移除。

---

## 3.10 console.debug 生产泄漏 ✅

**已完成**: `sessionStore.ts` 使用自定义 `logger.debug()` 替代 `console.debug`，日志级别可配置。
