# TODO: UX 增强改进

> 状态：**0/5 已完成** — 全部未开始（创建日期: 2026-06-08，更新: 2026-06-08）
>
> **含 1 个 Bug 修复 + 4 个功能改进**

---

## 🐛 6.0 SessionCard 状态不更新（Reactivity Bug）🔴

**问题**: Session 在后端已经变为 `[done]` 状态，但 `SessionCard` 面板上仍然显示为 `tool` 或 `starting` 等旧状态。只有点击展开面板后，状态才会更新为正确值。

**复现步骤**:
1. 启动 AgentPulse，启动一个 CC 会话
2. 观察 SessionCard 显示 `starting` → `tool`
3. 等待 CC 会话完成（status 变为 `completed`）
4. SessionCard 仍然显示 `tool`（而不是 `done`）
5. 点击 SessionCard 展开详情 → `ExpandedDetail` 显示正确状态 `done`
6. 折叠回去 → SessionCard 现在也显示 `done`

### 根因分析

**文件**: [apps/desktop/src/composables/useSessionDisplay.ts](../../apps/desktop/src/composables/useSessionDisplay.ts)
**文件**: [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)

**问题链路**:

```
SessionCard 组件 setup 时:
  props.session → 当前 snapshot 值 (plain AgentSession 对象)
    → useSessionDisplay(props.session)
      → session 参数 = 非响应式普通对象
        → computed(() => STATUS_COLORS[session.status])
          → 闭包捕获的 session 是 setup 时的快照
          → session.status 访问不触发 Vue 响应式追踪
          → computed 永远不重新求值 ❌
```

详细分析：

1. `SessionCard.vue:14` — `useSessionDisplay(props.session)` 在组件 setup 时调用，`props.session` 被解引用为当前值后传入函数参数
2. `useSessionDisplay.ts:6` — 函数参数 `session: AgentSession` 是一个**非响应式**的普通对象
3. `useSessionDisplay.ts:7` — `computed(() => STATUS_COLORS[session.status])` 的闭包捕获了这个普通 `session` 对象。由于 `session` 不是响应式代理，`session.status` 的访问**不会被 Vue 的响应式系统追踪**
4. 当父组件 `FloatingPanel` 的 `store.sessions` 被轮询更新为新数组后，`SessionCard` 的 prop `session` 在**模板层面**已经指向新对象，但 `useSessionDisplay` 的 `computed` 仍然引用初始快照
5. 点击展开时，`SessionCard` 被 `v-if` 卸载，`ExpandedDetail` **全新挂载** → setup 重新执行 → `useSessionDisplay` 获得当前最新的 `props.session` → 显示正确状态
6. 折叠后，`SessionCard` 重新挂载（全新 setup）→ 也显示正确状态

**关键证据**:
- 两个组件使用相同的 `useSessionDisplay` ，但 `ExpandedDetail` 在展开时才挂载（`v-if`），所以总是拿最新数据
- `SessionCard` 一直挂载（除展开状态外），`useSessionDisplay` 只在 setup 时执行一次
- 轮询 `/api/sessions` 返回的数据是正确的（后端状态已更新），但前端 computed 不重新求值

### 修复方案

**推荐方案 — 让 `useSessionDisplay` 接受响应式引用**:

修改 `useSessionDisplay` 签名，接收 `Ref<AgentSession>` 而非裸对象：

```diff
// useSessionDisplay.ts
+ import type { Ref, ComputedRef } from "vue";

- export function useSessionDisplay(session: AgentSession) {
+ export function useSessionDisplay(session: Ref<AgentSession> | ComputedRef<AgentSession>) {
-   const statusColor = computed(() => STATUS_COLORS[session.status]);
+   const statusColor = computed(() => STATUS_COLORS[session.value.status]);
-   const statusLabel = computed(() => STATUS_LABELS[session.status]);
+   const statusLabel = computed(() => STATUS_LABELS[session.value.status]);
-   const duration = computed(() => formatDuration(session.startedAt, session.completedAt));
+   const duration = computed(() => formatDuration(session.value.startedAt, session.value.completedAt));
  }
```

在 `SessionCard.vue` 和 `ExpandedDetail.vue` 中使用 `toRef`:

```diff
// SessionCard.vue
+ import { toRef } from "vue";

- const { statusColor, statusLabel, duration } = useSessionDisplay(props.session);
+ const sessionRef = toRef(props, 'session');
+ const { statusColor, statusLabel, duration } = useSessionDisplay(sessionRef);
```

`ExpandedDetail.vue` 同样修改。

**为什么这样修复**:
- `toRef(props, 'session')` 创建一个响应式引用，始终指向 `props` 上的最新 `session` 值
- 传入 composable 的 `session.value.status` 访问会被 Vue 追踪为响应式依赖
- 当父组件更新 prop 时，`toRef` 自动感知变化 → `computed` 重新求值 → UI 更新 ✅

**涉及文件**:
- [apps/desktop/src/composables/useSessionDisplay.ts](../../apps/desktop/src/composables/useSessionDisplay.ts) — 修改函数签名
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue) — 使用 `toRef`
- [apps/desktop/src/components/ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue) — 使用 `toRef`

---

## 6.1 滑动关闭已完成面板 🔴

**问题**: 当 session 状态变为 `[done]` 后，卡片会保留在面板中，直到对应的 CC 终端进程退出（约 5 秒）后才自动清理。用户无法手动提前关闭已完成的面板，面板空间被无用卡片占据。

**当前行为**:
- `FloatingPanel.vue` 的 `.session-list` 渲染 `SessionCard` 列表
- `SessionCard.vue` 仅支持 `@click` 展开详情，无滑动/关闭交互
- 已完成 session 靠 `process_checker.rs` 检测 PID 退出后自动清理

**建议方案**:

**方案 A — 滑动关闭（推荐）**:
- 在 `SessionCard.vue` 添加 touch/mouse 水平滑动检测
  - 使用 `@touchstart` / `@touchmove` / `@touchend` 事件
  - 同时支持鼠标拖拽（`@mousedown` / `@mousemove` / `@mouseup`）
- 仅在 `session.status === "completed"` 时启用滑动
- 滑动阈值：水平位移 > 60px 触发 dismiss
- 动画：卡片向右侧滑出 + 渐隐（复用现有 `<Transition name="slide">`）
- 滑动过程中显示红色背景或删除图标作为视觉反馈
- 触发后调用 store 方法移除该 session（前端移除 + 后端 DELETE API）

**方案 B — 添加关闭按钮（更简单）**:
- 在 `SessionCard.vue` 的 `[done]` 标签旁添加 `[x]` 按钮
- 点击后调用 store dismiss 方法
- 实现复杂度低，但占用卡片空间

**涉及文件**:
- [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue) — 添加 session list 动画过渡
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue) — 添加滑动/关闭逻辑
- [apps/desktop/src/stores/sessionStore.ts](../../apps/desktop/src/stores/sessionStore.ts) — 添加 dismissSession action
- [apps/desktop/src-tauri/src/commands.rs](../../apps/desktop/src-tauri/src/commands.rs) — 可能需要 `delete_session` Tauri command
- [apps/desktop/src-tauri/src/db.rs](../../apps/desktop/src-tauri/src/db.rs) — 如果后端没有 delete API

**实现要点**:
1. 创建 composable `useSwipeDismiss` 封装滑动检测逻辑
2. 仅在 `status === "completed"` 时激活滑动
3. 滑动进度绑定 CSS `transform: translateX()` 和 `opacity`
4. 超过阈值释放时触发 dismiss
5. 未超过阈值时回弹到原位

---

## 6.2 窗口可见但任务栏不显示 🔴

**问题**: 当前 AgentPulse 悬浮窗在任务栏显示为一个独立窗口图标。悬浮窗设计意图是轻量、不干扰工作流，任务栏图标增加了视觉噪音。

**当前行为**:
- `tauri.conf.json` 中 `app.windows[0]` 未配置 `skipTaskbar`
- 窗口在任务栏中正常显示

**建议方案**:
- 在 `tauri.conf.json` 的窗口配置中添加 `"skipTaskbar": true`
- Tauri 2 原生支持此属性，无需额外代码变更

**涉及文件**:
- [apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json) — 添加 `"skipTaskbar": true`

**注意**:
- `skipTaskbar: true` 后窗口只能通过系统托盘图标操作（最小化/恢复/退出）
- 与现有的 `"decorations": false` + `"alwaysOnTop": true` 配置不冲突
- 首次关闭时"最小化到托盘"的交互保持不变
- **平台行为差异**:
  - Windows: 窗口完全不在任务栏显示 ✅
  - macOS: 窗口不在 Dock 显示 ✅
  - Linux: 行为取决于窗口管理器（大多数支持）

**配置变更**:
```json
{
  "app": {
    "windows": [
      {
        // ... 现有配置 ...
        "skipTaskbar": true   // 新增
      }
    ]
  }
}
```

---

## 6.3 修改应用图标 🔴

**问题**: 当前应用图标是 Tauri 默认图标或占位图标，需要替换为 AgentPulse 品牌图标。

**当前图标文件**:
```
apps/desktop/src-tauri/icons/
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.icns          # macOS
├── icon.ico           # Windows
└── icon.png           # 通用
```

**建议方案**:
- 设计/获取新的 AgentPulse 品牌图标
- 生成各平台所需尺寸的图标文件
- 替换 `apps/desktop/src-tauri/icons/` 下的所有图标文件
- `tauri.conf.json` 中的 `bundle.icon` 和 `bundle.windows.nsis.installerIcon` 路径已正确配置，无需修改

**涉及文件**:
- [apps/desktop/src-tauri/icons/](../../apps/desktop/src-tauri/icons/) — 替换所有图标文件
- 可选：[apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json) — 如果新增/修改图标路径

**所需尺寸清单**:
| 平台 | 文件 | 尺寸 |
|------|------|------|
| Windows | `icon.ico` | 32x32, 48x48, 256x256 (三层) |
| Windows 安装器 | `icon.ico` | 同上 |
| macOS | `icon.icns` | 16x16 → 512x512@2x |
| Linux | `icon.png` | 512x512 |
| 通用 | `32x32.png` | 32x32 |
| 通用 | `128x128.png` | 128x128 |
| 通用 | `128x128@2x.png` | 256x256 |

**工具推荐**:
- 使用 `tauri icon` CLI 命令自动生成所有尺寸
- 或使用 ImageMagick / rsvg-convert 批量处理

**设计建议**:
- 考虑与 Catppuccin Mocha 深色主题协调
- 图标应体现 "pulse" / "agent monitoring" 概念
- 保持简洁，小尺寸下可辨识

---

## 6.4 done 状态颜色与 starting 相同 🔴

**问题**: `completed`（done）和 `starting` 状态使用了完全相同的颜色 `#89b4fa`（Catppuccin Blue），用户无法一眼区分"正在启动"和"已完成"的 session 卡片。

**当前颜色定义** ([agent.ts 第 63-72 行](../../apps/desktop/src/types/agent.ts#L63-L72)):
```typescript
export const STATUS_COLORS: Record<AgentStatus, string> = {
  starting: "#89b4fa",        // Catppuccin Blue
  completed: "#89b4fa",       // Catppuccin Blue ← 与 starting 相同！
  running: "#a6e3a1",         // Catppuccin Green
  tool_running: "#f9e2af",    // Catppuccin Yellow
  waiting_input: "#fab387",   // Catppuccin Peach
  waiting_permission: "#fab387", // Catppuccin Peach
  failed: "#f38ba8",          // Catppuccin Red
  unknown: "#6c7086",         // Catppuccin Overlay0
};
```

**建议方案**:
- 将 `completed` 改为 `#a6e3a1`（Catppuccin Green）— 与 `running` 同色
  - **优点**: 绿色直观表示"成功完成"，符合通用 UX 惯例
  - **缺点**: 与 `running` 颜色碰撞
- **推荐**: 将 `completed` 改为 `#94e2d5`（Catppuccin Teal）
  - 蓝绿色调，与蓝色 `starting` 和绿色 `running` 都不同
  - 传达"完成/稳定"的语义
  - 在当前深色主题下辨识度高
- 其他备选: `#cba6f7`（Catppuccin Mauve）

**涉及文件**:
- [apps/desktop/src/types/agent.ts](../../apps/desktop/src/types/agent.ts) — 修改 `STATUS_COLORS.completed` 值（第 69 行）

**注意**:
- `completed` 状态还用于 `needs_attention` 判断（`state_machine.rs` 第 64 行），修改前端颜色不影响后端逻辑
- 颜色变更会影响: `SessionCard.vue` 的 `borderLeftColor`、文字颜色，以及 `ExpandedDetail.vue` 的边框和标题颜色
- 建议同时检查 `useSessionDisplay.ts` composable 是否正确引用 `STATUS_COLORS`

**修改参考**:
```diff
- completed: "#89b4fa",
+ completed: "#94e2d5",  // Catppuccin Teal — 与 starting 区分
```
