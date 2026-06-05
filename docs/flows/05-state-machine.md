# 5. 状态机转换流程

## 涉及文件

- [state_machine.rs](../../apps/desktop/src-tauri/src/state_machine.rs) — 状态转换定义
- [lib.rs](../../apps/desktop/src-tauri/src/lib.rs) — `AgentStatus` 枚举定义

## 概述

StateMachine 是一个纯函数式状态机，负责根据当前状态和输入事件类型决定下一个状态。它没有内部状态，每次调用 `transition()` 都是纯计算。

## 状态定义 (AgentStatus)

```
┌──────────────┐
│   Starting   │ ← 初始状态（收到 SessionStart 事件）
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Running    │ ← 正常运行中
└──────┬───────┘
       │
       ├────→ ┌────────────────────┐
       │      │   ToolRunning      │ ← 正在执行工具 (PreToolUse)
       │      └─────────┬──────────┘
       │                │
       │                └────→ 返回 Running (PostToolUse)
       │
       ├────→ ┌────────────────────┐
       │      │  WaitingPermission │ ← 等待用户授权 (permission_prompt)
       │      └────────────────────┘
       │
       ├────→ ┌────────────────────┐
       │      │   WaitingInput     │ ← 等待用户输入 (idle_prompt)
       │      └────────────────────┘
       │
       ├────→ ┌────────────────────┐
       │      │    Completed       │ ← 终端状态 (Stop)
       │      └────────────────────┘
       │
       └────→ ┌────────────────────┐
              │     Failed         │ ← 终端状态 (PostToolUseFailure)
              └────────────────────┘
```

## 转换规则表

每一行的格式：`(当前状态, 事件类型) → 新状态`

### 全局覆盖规则（优先级最高）

| 当前状态 | 事件类型 | 新状态 | 说明 |
|----------|----------|--------|------|
| **任意** | `SessionStart` | `Starting` | SessionStart 总是重置到初始状态 |
| **任意** | `Stop` | `Completed` | Stop 总是标记完成 |
| **任意** | `Failure` | `Failed` | Failure 总是标记失败 |

### 特定转换规则

| 当前状态 | 事件类型 | 新状态 | 说明 |
|----------|----------|--------|------|
| `Starting` | **任意其他** | `Running` | 从 Starting 收到任何非全局覆盖事件 → Running |
| `Running` | `PreToolUse` | `ToolRunning` | 开始执行工具 |
| `WaitingInput` | `PreToolUse` | `ToolRunning` | 从等待输入转入工具执行 |
| `WaitingPermission` | `PreToolUse` | `ToolRunning` | 从等待授权转入工具执行 |
| `Completed` | `PreToolUse` | `ToolRunning` | 已完成 session 恢复（新工具调用） |
| `Failed` | `PreToolUse` | `ToolRunning` | 已失败 session 恢复 |
| `ToolRunning` | `PostToolUse` | `Running` | 工具执行完成，返回运行 |
| `Running` | `PermissionRequest` | `WaitingPermission` | 需要用户授权 |
| **任意** | `Notification` | `WaitingInput` | 通知事件 → 等待输入 |
| `Completed` | **任意其他** | `Running` | 唤醒已完成 session |
| `Failed` | **任意其他** | `Running` | 唤醒已失败 session |

### 默认规则

| 当前状态 | 事件类型 | 新状态 |
|----------|----------|--------|
| **任意** | **任意** | 保持不变 |

## 完整状态转换图

```
                        SessionStart
                    ┌──────────────────┐
                    │                  │
                    ▼                  │
              ┌──────────┐            │
              │ Starting │────────────┼── 任意其他事件 → Running
              └─────┬────┘            │
                    │                 │
                    │ 任意非全局事件   │
                    ▼                 │
              ┌──────────┐            │
      ┌───────│ Running  │◄───────────┘
      │       └─────┬────┘
      │             │
      │   ┌─────────┼──────────┐
      │   │         │          │
      │   ▼         ▼          ▼
      │ ┌──────┐ ┌──────┐ ┌──────────┐
      │ │Tool  │ │Wait  │ │Wait      │
      │ │Run   │ │Perm  │ │Input     │
      │ └──┬───┘ └──┬───┘ └────┬─────┘
      │    │        │          │
      │    │Post    │Pre       │Pre
      │    │ToolUse │ToolUse   │ToolUse
      │    │        │          │
      │    ▼        ▼          ▼
      │ ┌──────────┐ ┌──────┐ ┌──────┐
      │ │ 回到     │ │Tool  │ │Tool  │
      │ │ Running  │ │Run   │ │Run   │
      │ └──────────┘ └──────┘ └──────┘
      │
      ├──── Stop ────────→ ┌───────────┐
      │                    │ Completed │──┐
      │                    └───────────┘  │
      │                                   │ PreToolUse / 任意
      │                    ┌───────────┐  │ → 回到 Running
      └──── Failure ──────→│  Failed   │──┘
                           └───────────┘
```

## needs_attention 条件

`StateMachine::needs_attention()` 判断 session 是否需要用户关注：

```rust
fn needs_attention(status: &AgentStatus) -> bool {
    matches!(
        status,
        AgentStatus::WaitingInput
            | AgentStatus::WaitingPermission
            | AgentStatus::Completed
            | AgentStatus::Failed
    )
}
```

| 状态 | needs_attention | UI 效果 |
|------|-----------------|---------|
| `Starting` | ❌ | 正常卡片 |
| `Running` | ❌ | 正常卡片 |
| `ToolRunning` | ❌ | 正常卡片 |
| `WaitingInput` | ✅ | 脉冲动画 + 高亮 |
| `WaitingPermission` | ✅ | 脉冲动画 + 高亮 |
| `Completed` | ✅ | 脉冲动画 + done 标识 |
| `Failed` | ✅ | 脉冲动画 + failed 标识 |

## 代码位置映射

事件类型（hook_event_name → EventType → 触发的状态）的完整映射在 [event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs):

```rust
// lines 46-59
let (event_type, status) = match hook_event_name {
    "SessionStart"       → (SessionStart, Starting),
    "PreToolUse"         → (PreToolUse, ToolRunning),
    "PostToolUse"        → (PostToolUse, Running),
    "PostToolUseFailure" → (Failure, Failed),
    "Stop" | "SubagentStop" → (Stop, Completed),
    "Notification" → match notification_type {
        "permission_prompt" → (PermissionRequest, WaitingPermission),
        "idle_prompt"       → (Notification, WaitingInput),
        _                   → (Notification, Running),
    },
    "UserPromptSubmit"   → (Notification, Running),
    _                    → (Notification, Running),
};
```

状态转换逻辑在 [state_machine.rs](../../apps/desktop/src-tauri/src/state_machine.rs):

```rust
// lines 16-47
pub fn transition(&self, current: AgentStatus, event_type: &EventType) -> AgentStatus {
    match (current, event_type) {
        // 全局覆盖: SessionStart / Stop / Failure
        (_, SessionStart) => Starting,
        (_, Stop) => Completed,
        (_, Failure) => Failed,
        // Starting → Running
        (Starting, _) => Running,
        // PreToolUse 可从中断状态恢复
        (Running | WaitingInput | WaitingPermission | Completed | Failed, PreToolUse) => ToolRunning,
        (ToolRunning, PostToolUse) => Running,
        (Running, PermissionRequest) => WaitingPermission,
        (_, Notification) => WaitingInput,
        // 终态恢复
        (Completed | Failed, _) => Running,
        _ => current,
    }
}
```

## 关键设计决策

1. **无状态纯函数** — StateMachine 不持有状态，每次转换都是确定性计算
2. **全局覆盖规则** — `SessionStart`、`Stop`、`Failure` 可以从任意状态触发转换
3. **终态可恢复** — `Completed` 和 `Failed` 不是死胡同，收到新事件时可以转回 `Running`（支持"同一终端继续对话"场景）
4. **WaitingInput 通过 Notification 事件触发** — CC 的 Notification/idle_prompt 表示 CC 空闲等待用户输入
5. **WaitingPermission 通过 Notification/permission_prompt 触发** — CC 需要用户批准操作
