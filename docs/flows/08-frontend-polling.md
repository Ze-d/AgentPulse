# 8. 前端轮询与 UI 渲染流程

## 涉及文件

- [main.ts](../../apps/desktop/src/main.ts) — Vue 应用入口
- [App.vue](../../apps/desktop/src/App.vue) — 根组件
- [FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue) — 主面板
- [SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue) — 会话卡片
- [ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue) — 展开详情
- [sessionStore.ts](../../apps/desktop/src/stores/sessionStore.ts) — Pinia 状态管理
- [ipc.ts](../../apps/desktop/src/utils/ipc.ts) — Tauri IPC 调用
- [openActions.ts](../../apps/desktop/src/utils/openActions.ts) — 打开操作
- [sourceDisplay.ts](../../apps/desktop/src/utils/sourceDisplay.ts) — 来源显示
- [useSessionDisplay.ts](../../apps/desktop/src/composables/useSessionDisplay.ts) — Session 展示逻辑 (接收 Ref<AgentSession>)
- [useSwipeDismiss.ts](../../apps/desktop/src/composables/useSwipeDismiss.ts) — 滑动关闭 composable (touch + mouse)
- [agent.ts](../../apps/desktop/src/types/agent.ts) — TypeScript 类型 + 工具函数

## 整体架构

```
main.ts
  └─→ createApp(App)
        └─→ createPinia()
              └─→ app.mount("#app")
                    │
                    └─→ <App>
                          └─→ <FloatingPanel>
                                ├─→ onMounted → store.startPolling(2000)
                                ├─→ watch(sessions, adjustWindowSize)
                                │
                                ├─→ 空状态: "agentpulse is listening..."
                                │
                                └─→ 有数据:
                                      ├─→ <SessionCard v-for="session">
                                      │     └─→ 点击 → store.toggleExpand()
                                      └─→ <ExpandedDetail v-if="expanded">
```

## 数据流

```
                    Pinia sessionStore
                    ┌──────────────────────────────────────┐
                    │  state:                              │
                    │    sessions: AgentSession[]           │
                    │    expandedSessionId: string | null   │
                    │    error: string | null               │
                    │    isLoading: boolean                 │
                    │                                      │
                    │  getters:                            │
                    │    activeSessions                     │
                    │    attentionSessions                  │
                    │    expandedSession                    │
                    │                                      │
                    │  actions:                            │
                    │    fetchSessions() ←── 2s 定时器     │
                    │    startPolling(2000)                 │
                    │    stopPolling()                      │
                    │    toggleExpand(id)                   │
                    └───────────┬──────────────────────────┘
                                │
                    ┌───────────▼──────────────────────────┐
                    │  ipc.ts                              │
                    │  invoke<AgentSession[]>("get_sessions")│
                    └───────────┬──────────────────────────┘
                                │ Tauri IPC
                    ┌───────────▼──────────────────────────┐
                    │  Rust: commands::get_sessions()       │
                    │  → db.list_all_sessions()             │
                    │  → 返回 Vec<AgentSession>             │
                    └──────────────────────────────────────┘
```

## 轮询流程详解

```
sessionStore.startPolling(2000)
  │
  ├─→ stopPolling()                       // 先停止已有定时器
  ├─→ fetchSessions()                     // 立即执行一次
  │     │
  │     └─→ getSessions()
  │           └─→ invoke("get_sessions")  // Tauri IPC
  │                 │
  │                 ├─→ 成功:
  │                 │     ├─→ this.sessions = result
  │                 │     ├─→ this.error = null
  │                 │     └─→ this.isLoading = false
  │                 │
  │                 └─→ 失败:
  │                       ├─→ this.error = String(e)
  │                       └─→ this.isLoading = false
  │
  └─→ setInterval(fetchSessions, 2000)     // 每 2 秒执行
```

### 生命周期绑定

```
组件挂载 (onMounted):
  FloatingPanel → store.startPolling(2000)

组件卸载 (onUnmounted):
  FloatingPanel → store.stopPolling()
  (清理定时器，防止内存泄漏)
```

## 窗口自适应高度

```typescript
// FloatingPanel.vue
const HEADER_HEIGHT = 28;    // 头部高度
const CARD_HEIGHT = 34;      // 每张卡片高度
const PANEL_PADDING = 20;    // 面板内边距

function adjustWindowSize() {
  const expandedExtra = store.expandedSessionId ? 120 : 0;

  const contentHeight = store.sessions.length > 0
    ? HEADER_HEIGHT + store.sessions.length * CARD_HEIGHT + PANEL_PADDING + expandedExtra
    : 72;  // 空状态最小高度

  const height = Math.min(Math.max(contentHeight, 72), 420);
  //               └─ 不低于 72px ─┘  └─ 不高于 420px ─┘

  getCurrentWindow().setSize(new LogicalSize(320, height));
}

// 监听 sessions 数量和展开状态变化
watch(
  () => [store.sessions.length, store.expandedSessionId],
  () => adjustWindowSize(),
  { immediate: true }
);
```

## UI 状态展示

### SessionCard 显示内容

```
┌──────────────────────────────────────────────┐
│ █ cc > my-project          12m    running   │  ← card-row
│   Bash                                       │  ← card-row secondary
└──────────────────────────────────────────────┘
   │                                         │
   └─ border-left (状态颜色)                  └─ needsAttention?
                                                  → 脉冲动画 + 发光
```

数据来源 (useSessionDisplay composable，接收 Ref<AgentSession>):
```typescript
// 组件中使用 toRef 保持响应式链接
const sessionRef = toRef(props, "session");
const { statusColor, statusLabel, duration } = useSessionDisplay(sessionRef);

// composable 内部通过 .value 访问，Vue 正确追踪依赖
const statusColor = computed(() => STATUS_COLORS[session.value.status]);
```

### Swipe-to-Dismiss (completed 状态)

仅 `status === "completed"` 的卡片支持滑动关闭:
- **Touch**: `touchstart` → `touchmove` → `touchend`，向右滑动 > 80px 触发
- **Mouse**: `mousedown` (卡片) → `document mousemove/mouseup`，同样 80px 阈值
- **视觉**: 超过阈值后背景变红，显示 "✕ dismiss" 提示
- **行为**: 触发 → `dismissSession()` → 后端 `delete_session` + 前端卡片渐隐移除
- **回弹**: 未超过阈值时卡片弹簧回原位

### ExpandedDetail 显示内容

```
┌─────────────────────────────────────────┐
│ cc > my-project                  [ - ] │  ← 点击收起
├─────────────────────────────────────────┤
│ status        tool                     │  ← detail-grid
│ duration      12m 30s                  │
│ cwd           D:/projects/my-app       │
│ last tool     Bash                     │
│ transcript    /tmp/transcript.json     │
├─────────────────────────────────────────┤
│ $ Assistant: I'll run the tests...     │  ← message-block
├─────────────────────────────────────────┤
│ [open dir]  [transcript]               │  ← detail-actions
└─────────────────────────────────────────┘
```

### 状态颜色映射

| 状态 | 颜色 | Catppuccin 色名 |
|------|------|-----------------|
| `starting` | `#89b4fa` | Blue |
| `running` | `#a6e3a1` | Green |
| `tool_running` | `#f9e2af` | Yellow |
| `waiting_input` | `#fab387` | Peach |
| `waiting_permission` | `#fab387` | Peach |
| `completed` | `#89b4fa` | Blue |
| `failed` | `#f38ba8` | Red |
| `unknown` | `#6c7086` | Overlay0 |

### 来源缩写映射

```typescript
const SOURCE_ABBR = {
  "claude-code": "cc",
  "codex": "cx",
  "gemini": "gm",
  "copilot": "cp",
};
```

## 用户交互流程

### 点击卡片 → 展开/收起

```
SessionCard @click
  └─→ emit('click', sessionId)
        └─→ FloatingPanel.handleCardClick(sessionId)
              └─→ store.toggleExpand(sessionId)
                    └─→ expandedSessionId = (=== id) ? null : id
```

### 展开详情中的操作

```
ExpandedDetail
  │
  ├─→ [open dir] → emit('openDir', cwd)
  │     └─→ FloatingPanel.handleOpenDir(cwd)
  │           └─→ openPath(cwd)              // 通过 tauri-plugin-opener 打开文件管理器
  │
  └─→ [transcript] → emit('openTranscript', path)
        └─→ FloatingPanel.handleOpenTranscript(path)
              └─→ openPath(path)             // 打开 transcript 文件
```

### 刷新按钮

```
FloatingPanel header [↻]
  └─→ store.fetchSessions()
      (手动触发一次立即轮询)
```

### 关闭按钮

```
FloatingPanel header [_]
  └─→ handleClose()
        └─→ hideMainWindow()
              └─→ invoke("hide_main_window")
                    └─→ Rust: window.hide()
```

## 错误处理

前端错误展示为面板中的红色 banner：

```
┌──────────────────────────────────────────┐
│ Failed to connect to backend  [x]        │  ← error-banner
└──────────────────────────────────────────┘
```

- `fetchSessions()` 失败 → `store.error = String(e)`
- 用户可点击 `[x]` → `store.clearError()` 清除

## 空状态

```
无 sessions:
  初次加载: "$ _" (闪烁光标)
  加载完成: "$ agentpulse is listening..."
```
