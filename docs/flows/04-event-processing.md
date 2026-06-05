# 4. 服务端事件处理流程

## 涉及文件

- [event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs) — HTTP 服务器 + 事件处理
- [db.rs](../../apps/desktop/src-tauri/src/db.rs) — 数据库持久化
- [state_machine.rs](../../apps/desktop/src-tauri/src/state_machine.rs) — 状态转换引擎

## 概述

AgentPulse 内嵌了一个基于 `tiny_http` 的 HTTP 服务器（端口 17878），接收来自 `monitor_hook.py` 的 POST 请求。每个事件经过规范化、状态机转换、数据库持久化后返回响应。

## HTTP 路由表

| 方法 | 路径 | 说明 | 返回码 |
|------|------|------|--------|
| `POST` | `/api/events` | 接收 CC 事件 JSON | 201 成功 / 400 参数错误 / 500 服务端错误 |
| `GET` | `/api/sessions` | 返回所有 session 列表 | 200 |
| `GET` | `/api/health` | 健康检查 | 200 `{"status":"ok"}` |

## 完整处理流程 (POST /api/events)

```
POST /api/events
  │
  ├─→ 1. 读取请求体
  │     ├─→ request.as_reader().read_to_string(&mut body)
  │     ├─→ 失败 → 400 {"error": "failed to read body"}
  │     └─→ 成功 → 继续
  │
  ├─→ 2. JSON 解析
  │     ├─→ serde_json::from_str::<Value>(&body)
  │     ├─→ 失败 → 400 {"error": "invalid JSON: ..."}
  │     └─→ 成功 → EventServer::handle_event(&json)
  │           │
  │           ├─→ 3a. normalize_claude_code_event(raw)
  │           │     │
  │           │     ├─→ 提取字段:
  │           │     │     hook_event_name, session_id, cwd, transcript_path,
  │           │     │     message (优先) / last_assistant_message (回退),
  │           │     │     tool_name, notification_type, process_pid
  │           │     │
  │           │     ├─→ 推导 project_name = basename(cwd)
  │           │     │
  │           │     └─→ 映射 event_type + status:
  │           │           SessionStart       → SessionStart    / Starting
  │           │           PreToolUse         → PreToolUse      / ToolRunning
  │           │           PostToolUse        → PostToolUse     / Running
  │           │           PostToolUseFailure → Failure         / Failed
  │           │           Stop / SubagentStop → Stop           / Completed
  │           │           Notification:
  │           │             permission_prompt → PermissionRequest / WaitingPermission
  │           │             idle_prompt       → Notification     / WaitingInput
  │           │             其他              → Notification     / Running
  │           │           UserPromptSubmit   → Notification    / Running
  │           │           其他               → Notification    / Running
  │           │
  │           ├─→ 3b. db.get_session(&event.session_id)
  │           │     │
  │           │     ├─→ 已有 session（更新）:
  │           │     │     ├─→ state_machine.transition(old.status, event.event_type)
  │           │     │     ├─→ 计算 completed_at (Completed/Failed 时设置)
  │           │     │     ├─→ 合并 message / tool_name / transcript_path
  │           │     │     ├─→ needs_attention = StateMachine::needs_attention(new_status)
  │           │     │     └─→ pid = event.process_pid or old.pid（PID 回填）
  │           │     │
  │           │     └─→ 新 session（创建）:
  │           │           ├─→ 使用 event 的 status 作为初始状态
  │           │           ├─→ project_name = event.project_name || "unknown"
  │           │           ├─→ started_at = now
  │           │           └─→ needs_attention = StateMachine::needs_attention(status)
  │           │
  │           ├─→ 3c. db.upsert_session(&session)   // 写入/更新 sessions 表
  │           └─→ 3d. db.insert_event(&event)       // 写入 events 表
  │
  └─→ 4. 构建响应
        ├─→ 成功 → 201 {"event": {...}, "session": {...}}
        └─→ 错误 → 500 {"error": "..."}
```

## 事件规范化详解 (normalize_claude_code_event)

### 输入（来自 CC hook 的原始 JSON）

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc-123",
  "cwd": "D:/projects/my-app",
  "transcript_path": "/tmp/transcript.json",
  "tool_name": "Bash",
  "message": "Running command...",
  "notification_type": "",
  "process_pid": 12345
}
```

### 输出 (AgentEvent)

```json
{
  "id": "uuid-v4",
  "source": "claude-code",
  "sessionId": "abc-123",
  "cwd": "D:/projects/my-app",
  "projectName": "my-app",
  "eventType": "pre_tool_use",
  "status": "tool_running",
  "message": "Running command...",
  "toolName": "Bash",
  "transcriptPath": "/tmp/transcript.json",
  "createdAt": 1700000000000,
  "processPid": 12345
}
```

### message 字段优先级

```
event.message = raw["message"] || raw["last_assistant_message"]
```

CC 的 Stop 事件通常包含 `last_assistant_message` 而非 `message`。

## Session 的创建与更新策略

### 创建（数据库中没有该 session_id）

- 使用事件的状态作为初始状态
- `started_at` = `updated_at` = 当前时间
- `project_name` = cwd 的最后一个路径组件
- `completed_at` = None

### 更新（数据库中已有该 session_id）

- 通过状态机计算新状态
- `updated_at` = 当前时间
- 合并字段：新值优先，保留旧值作为回退
- `completed_at` = 终态时设为当前时间
- PID 回填：新 event 带了 PID 就用新的，否则保留旧的（防止 PID 丢失）

## GET /api/sessions 流程

```
GET /api/sessions
  │
  ├─→ db.lock() → 成功 → db.list_all_sessions()
  │     │                  └─→ SELECT * FROM sessions ORDER BY updated_at DESC
  │     │                  └─→ 200 [sessions JSON 数组]
  │     └─→ 失败（锁中毒）→ 500 {"error": "internal server error"}
  │
  └─→ Content-Type: application/json
```

## GET /api/health 流程

```
GET /api/health
  └─→ 200 {"status": "ok"}
```

（仅用于存活探测，不涉及数据库）

## 错误处理策略

| 错误类型 | HTTP 状态码 | 行为 |
|----------|------------|------|
| 请求体读取失败 | 400 | 返回错误 JSON |
| JSON 解析失败 | 400 | 返回错误 JSON + 原因 |
| 数据库锁中毒 | 500 | 返回错误 JSON + 日志 |
| 数据库操作失败 | 500 | 返回错误 JSON + 日志 |
| 未知路由 | 404 | 返回 `{"error": "not found"}` |

## 优雅关闭

EventServer 持有 `Arc<AtomicBool>` shutdown 信号：

```rust
// event_server.rs 循环中
for mut request in server.incoming_requests() {
    if self.shutdown.load(Ordering::Relaxed) {
        log::info!("event_server: shutdown signaled, stopping");
        break;
    }
    // ... 处理请求 ...
}
```

当 `shutdown()` 被调用时，循环自动退出，线程结束。
