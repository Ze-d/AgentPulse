# Bugfix: SessionCard 状态不更新（Vue 响应式断链）

> 日期: 2026-06-08 | 严重程度: Medium | 影响范围: 前端所有 SessionCard

## 症状

Session 在后端已变为 `[done]` 状态，但 SessionCard 面板仍显示旧状态（如 `tool` 或 `starting`）。只有点击卡片展开后，状态才更新为正确值。

## 根因

**Vue 3 响应式断链** — `useSessionDisplay` composable 接收非响应式裸对象。

```
SessionCard setup:
  props.session (reactive proxy)
    ↓ dereference to plain object
  useSessionDisplay(plainObject)
    ↓ closure captures non-reactive plainObject
  computed(() => STATUS_COLORS[plainObject.status])
    ↓ plainObject.status NOT tracked by Vue
  computed NEVER re-evaluates ❌
```

关键链路：
1. `SessionCard.vue:14` — `useSessionDisplay(props.session)` 解引用 prop 为当前快照值
2. `useSessionDisplay.ts:5` — 函数参数 `session: AgentSession` 是普通对象，非响应式
3. `useSessionDisplay.ts:7` — `computed` 闭包捕获普通 `session`，`session.status` 不被 Vue 追踪
4. 父组件轮询更新 `store.sessions` → prop 更新 → 但 computed 已锁定初始快照

## 修复

**3 个文件修改**:

| 文件 | 变更 |
|------|------|
| `useSessionDisplay.ts` | 参数 `AgentSession` → `Ref<AgentSession> \| ComputedRef<AgentSession>`，内部 `.status` → `.value.status` |
| `SessionCard.vue` | `props.session` → `toRef(props, "session")` 传入 composable |
| `ExpandedDetail.vue` | 同上 |

**原理**: `toRef` 创建始终指向 prop 最新值的响应式引用 → `.value.status` 被 Vue 追踪 → prop 变化 → computed 重算 → UI 更新

## 验证

- `vue-tsc --noEmit` — 0 errors
- `vitest` — 18/18 passed
- 手动测试: SessionCard 状态随轮询实时更新，无须点击展开

## 经验教训

- Composable 接收对象参数时应使用 `Ref<T>` 而非裸 `T`，保持响应式链接
- `props.xxx` 传入函数时会丢失响应式 — 使用 `toRef(props, 'xxx')` 保持追踪
- `v-if` 挂载/卸载行为（ExpandedDetail）掩盖了响应式 bug — 展开时"看起来修好了"只是巧合
