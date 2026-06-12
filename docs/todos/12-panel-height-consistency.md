# TODO: 统一不同状态下 SessionCard 面板高度

> 状态：**待修复** 🔲（创建日期: 2026-06-10）
>
> 类型：Bug 修复

---

## 问题描述

`SessionCard` 在不同状态下渲染的行数不同：
- **有 `lastToolName` 的状态**（`running`, `tool_running`, `waiting_input`, `waiting_permission`）：渲染两行（主行 + 工具名行）
- **无 `lastToolName` 的状态**（`starting`, `completed`, `failed`, `unknown`）：仅渲染一行（主行）

这导致 starting 状态的卡片比其他状态的卡片矮约 14px，当面板中混有不同状态的卡片时：

1. **窗口高度计算不准确** — `FloatingPanel.vue` 的 `adjustWindowSize()` 使用固定 `CARD_HEIGHT = 34`，但实际卡片高度在 ~26px（单行）到 ~41px（双行）之间变化
2. **底部边界遮挡** — 窗口高度按固定值计算后，单行卡片的实际总高度小于预期，导致面板底部出现多余空白；反之双行卡片过多时，底部内容被窗口下边界裁切遮挡

---

## 根因分析

### 1. 卡片高度不一致

**文件**: [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue#L104-L106)

```html
<div v-if="session.lastToolName" class="card-row secondary">
  <span class="tool">{{ session.lastToolName }}</span>
</div>
```

`v-if` 导致 `starting` 等状态的卡片 DOM 中不存在 `.card-row.secondary`，卡片内容高度减少：

| 状态 | 行数 | 实际高度（估算） |
|------|------|-----------------|
| `starting` | 1 | ~26px |
| `completed` | 1 | ~26px |
| `failed` | 1 | ~26px |
| `unknown` | 1 | ~26px |
| `running` | 2 | ~41px |
| `tool_running` | 2 | ~41px |
| `waiting_input` | 2 | ~41px |
| `waiting_permission` | 2 | ~41px |

### 2. 窗口高度计算使用固定值

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue#L14)

```typescript
const CARD_HEIGHT = 34;  // 固定值，不区分单行/双行
```

`adjustWindowSize()` 中 `store.sessions.length * CARD_HEIGHT` 对所有卡片使用相同的 34px，与实际情况偏差：

- 全是 starting 卡片时（单行 ~26px）：实际内容比计算值矮 ~8px/card → 底部出现空白
- 全是 running 卡片时（双行 ~41px）：实际内容比计算值高 ~7px/card → 底部被裁切

---

## 建议方案

### 方案 A — 统一卡片最小高度（推荐）

在 `SessionCard` 的样式中添加 `min-height`，确保所有卡片具有相同高度：

```css
.session-card {
  /* ... 现有样式 ... */
  min-height: 34px;  /* 或 36px，匹配 HEADER_HEIGHT 比例 */
  display: flex;
  flex-direction: column;
  justify-content: center;
}
```

**优点**:
- 改动最小，仅 CSS 一行
- 不需要改动 JS 逻辑
- 单行卡片内容垂直居中，视觉自然
- `CARD_HEIGHT = 34` 的计算立即变准确

**缺点**:
- 单行卡片有少量垂直空白（约 4px 上下各半）

### 方案 B — 始终渲染 secondary 行占位

将 `v-if` 改为 `v-show` 或始终渲染但用占位内容：

```html
<div class="card-row secondary">
  <span class="tool">{{ session.lastToolName || ' ' }}</span>
  <!--   = non-breaking space, 保持行高 -->
</div>
```

**优点**:
- 不需要硬编码 `min-height`
- 高度自然一致

**缺点**:
- 需要确保空行不改变间距（`margin-top: 1px` 仍然生效）

### 方案 C — 动态 CARD_HEIGHT

在 `adjustWindowSize()` 中根据每个 session 的实际状态动态计算高度：

```typescript
function getCardHeight(session: AgentSession): number {
  return session.lastToolName ? 41 : 28;
}

const contentHeight = store.sessions.reduce(
  (sum, s) => sum + getCardHeight(s), 0
) + HEADER_HEIGHT + PANEL_PADDING + expandedExtra;
```

**优点**:
- 窗口高度更精确
- 不需要改变卡片样式

**缺点**:
- 需要在多处维护高度映射
- 引入了 JS 和 CSS 之间的隐性耦合（CSS 改了高度 JS 也得改）

---

## 推荐实施

**方案 A（CSS min-height）作为主要修复 + 微调 CARD_HEIGHT 常量**：

1. `SessionCard.vue` 样式: 添加 `min-height: 36px`
2. `FloatingPanel.vue`: 将 `CARD_HEIGHT` 从 `34` 改为 `40`（含 `margin-bottom: 4px`）
3. 验证各种状态组合下的窗口高度表现

---

## 涉及文件

| 文件 | 变更内容 |
|------|---------|
| [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue#L146-L156) | 添加 `min-height` 样式 |
| [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue#L14) | 微调 `CARD_HEIGHT` 常量 |

---

## 验证项

- [ ] 所有状态的卡片在面板中高度一致
- [ ] 面板底部边界不再被遮挡
- [ ] 单行卡片内容仍垂直居中显示
- [ ] 双行卡片布局不变
- [ ] 混合状态（starting + running + completed）下窗口高度正确
- [ ] 展开 `ExpandedDetail` 后窗口高度动态调整正确
- [ ] `vue-tsc --noEmit` 0 错误
