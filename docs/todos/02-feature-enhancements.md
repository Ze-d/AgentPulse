# ~~TODO: 功能增强~~ ✅ 全部完成

> 状态：**全部完成** — 11/11 项已实现（验证日期: 2026-06-07）

---

## 2.1 数据库持久化存储 ✅

**已完成**: `db.rs` L19-27 添加 `Database::new(path)` 文件持久化方法，schema 使用 `CREATE TABLE IF NOT EXISTS` 自动迁移，L334-359 添加 `cleanup_old_sessions` 定期清理。

---

## 2.2 needs_attention 可视化 ✅

**已完成**: `SessionCard.vue` L87-99 的 `.session-card.attention` 使用 `attention-pulse` 动画 + `box-shadow` 脉冲效果。

---

## 2.3 Session 卡片展开/折叠动画 ✅

**已完成**: `FloatingPanel.vue` L131-139 使用 `<Transition name="slide">` 包裹 ExpandedDetail，L320-329 定义 slide transition。

---

## 2.4 加载状态指示器 ✅

**已完成**: `sessionStore.ts` L13 添加 `isLoading: boolean` 状态，L49 在 fetch 后置为 false。`FloatingPanel.vue` L125 加载时显示闪烁 `_` 光标。

---

## 2.5 手动刷新按钮 ✅

**已完成**: `FloatingPanel.vue` L102 添加 `↻` 刷新按钮，点击触发 `store.fetchSessions()`。

---

## 2.6 错误 banner 可关闭 ✅

**已完成**: `FloatingPanel.vue` L117 添加 `×` 关闭按钮，`sessionStore.ts` L75-77 添加 `clearError()` action。

---

## 2.7 用户友好的空状态文案 ✅

**已完成**: 空状态文案改为 `$ agentpulse is listening...`，加载时显示闪烁光标。

---

## 2.8 关闭按钮语义优化 ✅

**已完成**: `FloatingPanel.vue` L108 `title="Minimize to tray"`，使用 `_` 图标。

---

## 2.9 项目名 tooltip ✅

**已完成**: `SessionCard.vue` L25 添加 `:title="session.projectName"`。

---

## 2.10 "Open dir" / "Transcript" 失败反馈 ✅

**已完成**: `FloatingPanel.vue` L38-41 和 L48-51 均使用 `store.error = String(e)` 显示错误。

---

## 2.11 多源 Agent 支持 ✅

**已完成**: `lib.rs` L20-27 的 `AgentSource` 枚举扩展为 `ClaudeCode`, `Codex`, `Gemini`, `Copilot` 四个变体。`db.rs` L644-657 测试验证所有变体序列化/反序列化。
