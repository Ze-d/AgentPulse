# TODO: 支持 starting 状态面板滑动删除

> 状态：**待实现** 🔲（创建日期: 2026-06-10）
>
> 类型：功能增强

---

## 问题描述

当 VSCode 启动时，如果配置了 Claude Code 扩展，VSCode 会在启动的同时自动打开一个 Claude Code 终端。AgentPulse 通过 hook 捕获到 `session_start` 事件后，会在悬浮面板中显示一个 `starting` 状态的卡片。

但用户不一定需要使用这个 Claude Code 实例。如果用户在 VSCode 中直接关闭了该 CC 终端（或 CC 进程因某种原因卡在 starting 状态），AgentPulse 面板中的 `starting` 卡片会一直残留，无法手动关闭。

**当前行为**:
- `SessionCard.vue` 第 21 行: `canSwipe` 仅在 `status === "completed"` 时为 `true`
- `starting` 状态的卡片不支持滑动删除，用户只能等待后端 `process_checker` 检测到 PID 退出后自动清理
- 如果 CC 进程根本没启动成功，或者 PID 无法被检测到，卡片会永久残留

**期望行为**:
- `starting` 状态的卡片也支持滑动删除
- 用户可以手动关闭不打算使用的会话卡片

---

## 根因分析

**文件**: [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue#L21)

```typescript
const canSwipe = props.session.status === "completed";
```

`canSwipe` 的判定过于严格，只允许 `completed` 状态滑动。`starting` 是一个过渡状态，但可能因为以下原因卡住：
1. VSCode CC 终端被用户手动关闭，但 hook 没来得及发送后续事件
2. CC 启动失败（如认证问题），但 `failure` 事件未被正确捕获
3. 多个 CC 实例同时启动，用户只想保留其中一个

---

## 建议方案

### 方案 A — 扩展 `canSwipe` 到非活跃状态（推荐）

将 `canSwipe` 从仅 `completed` 扩展到所有"可安全删除"的状态：

```diff
// SessionCard.vue
- const canSwipe = props.session.status === "completed";
+ const DISMISSABLE_STATUSES: AgentStatus[] = ["completed", "starting", "failed", "unknown"];
+ const canSwipe = DISMISSABLE_STATUSES.includes(props.session.status);
```

**不可滑动删除的状态**（这些是活跃进行中的会话）:
- `running` — 正在执行中
- `tool_running` — 工具执行中
- `waiting_input` — 等待用户输入
- `waiting_permission` — 等待权限确认

**可滑动删除的状态**:
- `completed` — 已完成（已有功能）
- `starting` — 启动中/卡在启动（新增）
- `failed` — 失败（新增）
- `unknown` — 未知（新增）

### 方案 B — 所有状态均可滑动删除

完全移除 `canSwipe` 限制，所有状态的卡片均可滑动删除。风险是用户可能误删正在活跃使用的会话。

---

## 涉及文件

| 文件 | 变更内容 |
|------|---------|
| [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue#L21) | 扩展 `canSwipe` 判定条件 |
| [apps/desktop/src/composables/useSwipeDismiss.ts](../../apps/desktop/src/composables/useSwipeDismiss.ts) | 可能需要支持不同的滑动颜色/提示（starting 用黄色而非红色） |

---

## 实现要点

1. 定义 `DISMISSABLE_STATUSES` 常量（或在 `types/agent.ts` 中导出）
2. 修改 `canSwipe` 使用该常量
3. 可选：为不同状态的滑动删除提供不同的视觉反馈
   - `completed` → 红色背景 + "✕ dismiss"（现有行为）
   - `starting` → 黄色背景 + "✕ cancel"
   - `failed` → 红色背景 + "✕ dismiss"
4. 后端 `dismissSession` 已支持删除任意状态的 session（`db.rs` 的 `delete_session` 按 `session_id` 删除，不检查状态），无需后端改动
5. 确认 `process_checker.rs` 的 PID 检测逻辑不会与手动删除产生竞态条件

---

## 验证项

- [ ] `starting` 状态卡片可滑动删除
- [ ] `failed` / `unknown` 状态卡片可滑动删除
- [ ] `running` / `tool_running` / `waiting_input` / `waiting_permission` 状态卡片不可滑动删除
- [ ] 删除后 `sessionStore.sessions` 中移除对应条目
- [ ] 删除后后端数据库记录被清除
- [ ] 窗口高度在删除后正确重新计算
- [ ] `vue-tsc --noEmit` 0 错误
- [ ] `vitest` 全部通过
