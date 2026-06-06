# TODO: 可访问性改进 ❌ 全部未完成

> 状态：**0/7 已完成** — 所有可访问性任务均未开始（验证日期: 2026-06-07）
>
> 键盘导航、屏幕阅读器支持、语义化

---

## 4.1 Session 卡片不可键盘操作 🔴

**问题**: `SessionCard.vue` 使用 `<div @click>`，无 `tabindex`、无 `role`、无 `onkeydown`。键盘用户无法选中、激活卡片。

**文件**: [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)

**建议**:
- 改为 `<button>` 或添加 `role="button" tabindex="0"`
- 添加 `@keydown.enter` 和 `@keydown.space` handler
- 添加 `aria-label` 如 "agent-pulse session: running, 3m 24s"

---

## 4.2 仅通过颜色传达状态 🔴

**问题**: 状态仅通过 `border-left-color` 和文字 `color` 区分。屏幕阅读器用户不知道 session 是什么状态。

**建议**:
- 使用 `aria-label` 在卡片上包含状态信息
- 或使用 `aria-live="polite"` 区域动态播报状态变更
- 考虑添加状态 icon 作为辅助视觉指示

---

## 4.3 无 ARIA live region 🟡

**问题**: Session 每 2 秒轮询刷新，列表静默更新。屏幕阅读器用户无感知。

**建议**: 在 session list 容器添加 `aria-live="polite" aria-atomic="false"`。

---

## 4.4 ExpandedDetail 按钮无语义标签 🟡

**问题**: "open dir" 和 "transcript" 按钮无 `aria-label`，屏幕阅读器听不到上下文。

**文件**: [apps/desktop/src/components/ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue)

**建议**: 添加 `aria-label="Open project directory for agent-pulse"` 和 `aria-label="Open transcript for agent-pulse"`。

---

## 4.5 折叠按钮无 label 🟡

**问题**: ExpandedDetail 的 `[ - ]` 按钮无 `aria-label`。

**建议**: 添加 `aria-label="Collapse session detail"`。

---

## 4.6 展开/折叠焦点管理 🟢

**问题**: 展开卡片时焦点停留在已隐藏的卡片上；折叠时焦点不回到卡片。

**建议**: 展开后自动聚焦到 ExpandedDetail 的第一个可交互元素；折叠后聚焦回 SessionCard。

---

## 4.7 关闭按钮缺少 aria-label 🟢

**问题**: 关闭按钮仅有 `title="Close"`，不符合 WCAG 规范。

**建议**: 添加 `aria-label="Minimize AgentPulse to system tray"`。
