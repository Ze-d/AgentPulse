# 10 — 可访问性改进

**状态：** 待规划  
**优先级：** 高  
**创建日期：** 2026-06-10

> 整合自 [[04-accessibility]] — 原始 7 项 sub-tasks 全部未开始。本文档将其重组为可执行的工作项。

---

## 10.1 SessionCard 键盘可操作性 🔴

**问题**: `SessionCard.vue` 使用 `<div @click>`，无 `tabindex`、无 `role`、无键盘事件。键盘用户完全无法使用。

**涉及文件**:
- `apps/desktop/src/components/SessionCard.vue`

**改动**:
1. 添加 `role="button"`、`tabindex="0"`
2. 添加 `@keydown.enter` 和 `@keydown.space.prevent` handler（触发点击）
3. 添加 `aria-label`，如 `"AgentPulse session: {status}, {duration}"`
4. 添加 `:aria-expanded="isExpanded"` 表示当前展开状态

```vue
<div
  role="button"
  tabindex="0"
  :aria-label="`AgentPulse session: ${statusLabel}, ${duration}`"
  :aria-expanded="isExpanded"
  @keydown.enter="handleClick"
  @keydown.space.prevent="handleClick"
  @click="handleClick"
>
```

---

## 10.2 ExpandedDetail 按钮语义标签 🔴

**问题**: "open dir" 和 "transcript" 按钮无 `aria-label`，屏幕阅读器无法理解其用途。

**涉及文件**:
- `apps/desktop/src/components/ExpandedDetail.vue`

**改动**:
- "open dir" 按钮: `aria-label="Open project directory"`
- "transcript" 按钮: `aria-label="Open transcript"`
- 折叠按钮 `[ − ]`: `aria-label="Collapse session detail"`
- 确保所有按钮使用 `<button>` 元素（而非 `<div @click>`）

---

## 10.3 状态仅靠颜色传达 🟡

**问题**: 状态通过 `border-left-color` 和文字 `color` 区分，色盲用户和屏幕阅读器用户无法感知。

**涉及文件**:
- `apps/desktop/src/components/SessionCard.vue`
- `apps/desktop/src/components/ExpandedDetail.vue`

**建议**:
- 在卡片 `aria-label` 中包含状态文字（见 10.1）
- 考虑添加状态小图标（如 △ 等待、✓ 完成、✗ 失败）作为辅助视觉指示
- `ExpandedDetail` 的状态标题使用 `role="status"` 标记

---

## 10.4 ARIA Live Region（动态更新通知）🟡

**问题**: Session 每 N 秒轮询刷新，列表静默更新。屏幕阅读器用户完全不知道数据发生了变化。

**涉及文件**:
- `apps/desktop/src/components/FloatingPanel.vue`（session 列表容器）

**改动**:
在 session 列表的外层容器添加：
```html
<div aria-live="polite" aria-atomic="false" aria-relevant="additions removals">
  <SessionCard v-for="session in sessions" ... />
</div>
```

- `aria-live="polite"`: 不打断当前朗读，等空闲时播报变更
- `aria-atomic="false"`: 只播报变更的部分，不重读整个列表
- `aria-relevant="additions removals"`: 仅在新增/移除时通知（减少噪音）

---

## 10.5 展开/折叠焦点管理 🟢

**问题**: 展开卡片时焦点停留在已被隐藏的卡片上；折叠时焦点不回到卡片。

**涉及文件**:
- `apps/desktop/src/components/SessionCard.vue`
- `apps/desktop/src/components/ExpandedDetail.vue`

**改动**:
- 展开时：用 `nextTick` + `focus()` 将焦点移到 `ExpandedDetail` 的第一个可交互元素
- 折叠时：用 `nextTick` + `focus()` 将焦点回到对应的 `SessionCard`
- `ExpandedDetail` 中按 `Escape` 键折叠并恢复焦点

---

## 10.6 关闭按钮无 `aria-label` 🟢

**问题**: 关闭按钮仅有 `title="Close"`，不符合 WCAG 2.1 规范（`title` 属性不被屏幕阅读器可靠读取）。

**涉及文件**:
- `apps/desktop/src/components/FloatingPanel.vue`

**改动**:
```html
<button aria-label="Minimize AgentPulse to system tray" title="Close">
```

---

## 10.7 颜色对比度审查 🟢

**问题**: 虽然 Catppuccin Mocha 调色板通常对比度不错，但未经过 WCAG AA 验证。

**建议**:
- 使用 Chrome DevTools 或 axe DevTools 检查所有文本/背景组合的对比度
- 重点检查：
  - 状态文字颜色 vs 深色背景（`#1e1e2e`）
  - 灰色辅助文字（`#6c7086` Overlay0）vs 背景 — 可能不达标
  - 黄色状态文字（`#f9e2af`）vs 背景 — 浅色在深色背景上可读性需验证

---

## 实施顺序建议

1. **10.1 + 10.2** — 键盘操作 + 语义标签（改动集中、收益最大）
2. **10.4** — ARIA live region（一行改动）
3. **10.5** — 焦点管理
4. **10.3 + 10.6 + 10.7** — 视觉/语义补充

---

## 关联

- [[04-accessibility]] — 原始可访问性任务清单
- [[08-infra-improvements]] — 基础设施改进
