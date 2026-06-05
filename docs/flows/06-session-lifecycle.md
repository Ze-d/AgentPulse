# 6. Session 生命周期流程

## 涉及文件

- [event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs) — Session 创建/更新
- [state_machine.rs](../../apps/desktop/src-tauri/src/state_machine.rs) — 状态转换
- [db.rs](../../apps/desktop/src-tauri/src/db.rs) — 数据持久化
- [process_checker.rs](../../apps/desktop/src-tauri/src/process_checker.rs) — Session 清理

## 完整生命周期

```
                               CC 启动，打开终端
                                     │
                                     ▼
                         ┌─────────────────────┐
                         │ 1. CREATE           │
                         │ SessionStart 事件   │
                         │ → status: Starting  │
                         │ → 记录 PID          │
                         │ → 分配 session_id   │
                         └─────────┬───────────┘
                                   │
                                   ▼
                         ┌─────────────────────┐
                         │ 2. RUNNING          │
                         │ 第一个非全局事件    │
                         │ → Starting→Running  │
                         └─────────┬───────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
          ┌────────────┐ ┌────────────┐ ┌─────────────┐
          │ 3a. TOOL   │ │ 3b. WAIT   │ │ 3c. WAIT    │
          │  RUNNING   │ │  PERM      │ │  INPUT      │
          │ PreToolUse │ │ perm_prompt│ │ idle_prompt │
          └─────┬──────┘ └─────┬──────┘ └──────┬──────┘
                │              │               │
                │ PostToolUse  │ PreToolUse    │ PreToolUse
                ▼              ▼               ▼
          ┌────────────┐ ┌────────────┐ ┌─────────────┐
          │ 回到       │ │ 回到       │ │ 回到        │
          │ Running    │ │ Running    │ │ Running     │
          └────────────┘ └────────────┘ └─────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              │
          ┌────────────┐ ┌────────────┐          │
          │ 4a. DONE   │ │ 4b. FAIL   │          │
          │ Completed  │ │  Failed    │          │
          │ (Stop事件) │ │ (失败事件) │          │
          └─────┬──────┘ └─────┬──────┘          │
                │              │                 │
                │ PreToolUse   │ PreToolUse      │
                │ 或其他事件   │ 或其他事件      │
                ▼              ▼                 │
          ┌────────────┐ ┌────────────┐          │
          │ 5. RECOVER │ │ 5. RECOVER │ ◄────────┘
          │ 回到Running│ │ 回到Running│  同一终端继续对话
          └────────────┘ └────────────┘
                                   │
                                   ▼
                         ┌─────────────────────┐
                         │ 6. CLEANUP          │
                         │ CC 进程退出         │
                         │ process_checker     │
                         │ 检测 PID 不存在     │
                         │ → delete_session()  │
                         │ → 卡片从 UI 消失    │
                         └─────────────────────┘
```

## 各阶段详解

### 阶段 1: CREATE — Session 创建

**触发**: CC 启动，hook 触发 `SessionStart` 事件

**数据操作**:
```sql
-- db.upsert_session() 插入新记录
INSERT INTO sessions (session_id, source, cwd, project_name, status,
                      started_at, updated_at, needs_attention, pid)
VALUES (?, 'claude-code', ?, ?, 'starting', now, now, false, ?);
```

**关键属性**:
- `session_id` — CC 分配的唯一标识
- `project_name` — 从 `cwd` 的 basename 推导
- `pid` — `monitor_hook.py` 通过进程树遍历获取的 CC node.exe PID
- `status` — `Starting`

### 阶段 2: RUNNING — 正常运行

**触发**: `Starting` 状态收到任意非全局覆盖事件

**状态转换**:
```
Starting + PreToolUse / Notification / UserPromptSubmit / ... → Running
```

### 阶段 3a: TOOL RUNNING — 工具执行中

**触发**: CC 调用工具（Read, Bash, Write...）

**数据更新**:
```sql
UPDATE sessions SET
  status = 'tool_running',
  last_tool_name = 'Bash',    -- 工具名
  updated_at = now
WHERE session_id = ?;
```

**返回 Running**: CC 完成工具调用后触发 `PostToolUse` → 状态回到 `Running`

### 阶段 3b: WAITING PERMISSION — 等待授权

**触发**: CC 需要用户批准操作（如执行 shell 命令）

**状态转换**: `Running + PermissionRequest → WaitingPermission`

**needs_attention = true** → UI 卡片脉冲高亮

### 阶段 3c: WAITING INPUT — 等待输入

**触发**: CC 空闲，等待用户输入新消息

**状态转换**: `任意 + Notification → WaitingInput`

### 阶段 4a/4b: 终端状态

**Completed**:
- 触发: CC 正常退出（`Stop` / `SubagentStop`）
- `completed_at` 设为当前时间
- `needs_attention = true`
- 卡片保留，显示 `[done]` 状态

**Failed**:
- 触发: 工具执行失败（`PostToolUseFailure`）
- `completed_at` 设为当前时间
- `needs_attention = true`
- 卡片保留，显示 `[failed]` 状态

### 阶段 5: RECOVER — 终端状态恢复

**场景**: 同一 CC 终端中开始新一轮对话

**触发**: `Completed`/`Failed` 的 session 收到新事件

**状态转换**:
```
Completed + PreToolUse → ToolRunning
Failed + PreToolUse → ToolRunning
Completed/Failed + 任意其他事件 → Running
```

这意味着卡片会从 `[done]` 自动变回活跃状态，无需用户干预。

### 阶段 6: CLEANUP — Session 清理

**触发**: 后台 `process_checker` 检测 CC 进程已退出

**条件**: session 有 PID 且 PID 不存在于系统中

**数据操作**:
```sql
DELETE FROM events WHERE session_id = ?;
DELETE FROM sessions WHERE session_id = ?;
```

**UI 效果**: 前端下次轮询时卡片消失

**重要**: 只有**非终端状态**的 session 才被 process_checker 清理：
```rust
// 终端状态跳过清理，保留展示
fn is_active_status(status: &AgentStatus) -> bool {
    !matches!(status, Completed | Failed | Unknown)
}
```

## PID 回填机制

如果 `SessionStart` 事件丢失（没有 PID），但后续事件带有 PID：

```rust
// event_server.rs handle_event() 中
pid: event.process_pid.or(old.pid),
```

`old.pid` 是 `None` → 用新 PID 填充。后续 `process_checker` 就可以检测到该 session。

如果整个生命周期都没有 PID，session 永远不会被 process_checker 自动清理（需重启应用清除）。

## 时间戳管理

| 字段 | 设置时机 | 说明 |
|------|----------|------|
| `started_at` | 创建时 | 首次收到该 session 事件的时间 |
| `updated_at` | 每次事件 | 最后收到事件的时间 |
| `completed_at` | 进入终态时 | 首次变为 Completed/Failed 的时间 |
| `created_at` (event) | 事件创建时 | 事件记录的时间戳 |
