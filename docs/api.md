# AgentPulse HTTP API

AgentPulse 提供一个本地 HTTP 服务器，用于接收来自 Claude Code / Codex hook 的事件。第三方开发者可通过此 API 将自己的 agent 工具接入 AgentPulse 监控系统。

## 基础信息

- **Base URL**: `http://127.0.0.1:{port}`（默认端口 `17878`，可通过 `AGENTPULSE_PORT` 环境变量或 `config.json` 配置）
- **Content-Type**: `application/json`
- **字符编码**: UTF-8

## 端点

### POST /api/events

接收一个 hook 事件，规范化后更新 session 状态并写入数据库。

**Request Body** (JSON):

```json
{
  "hook_event_name": "SessionStart | PreToolUse | PostToolUse | PostToolUseFailure | Notification | Stop | PermissionRequest | UserPromptSubmit",
  "session_id": "<uuid>",
  "cwd": "<working directory>",
  "transcript_path": "<optional transcript path>",
  "message": "<optional message>",
  "last_assistant_message": "<optional fallback message>",
  "tool_name": "<optional tool name>",
  "notification_type": "<permission_prompt | idle_prompt>",
  "process_pid": "<optional process PID>",
  "agent_source": "<claude-code | codex>"
}
```

**Response** `201 Created`:

```json
{
  "event": {
    "id": "<uuid>",
    "source": "claude-code | codex",
    "sessionId": "<session_id>",
    "cwd": "<cwd>",
    "projectName": "<project name>",
    "eventType": "session_start | pre_tool_use | post_tool_use | permission_request | notification | stop | failure",
    "status": "starting | running | tool_running | waiting_input | waiting_permission | completed | failed",
    "message": "<optional message>",
    "toolName": "<optional tool name>",
    "transcriptPath": "<optional transcript path>",
    "createdAt": 1700000000000,
    "processPid": 12345
  },
  "session": {
    "sessionId": "<session_id>",
    "source": "claude-code | codex",
    "cwd": "<cwd>",
    "projectName": "<project name>",
    "status": "...",
    "startedAt": 1700000000000,
    "updatedAt": 1700000000000,
    "completedAt": null,
    "lastMessage": "<optional>",
    "lastToolName": "<optional>",
    "transcriptPath": "<optional>",
    "needsAttention": false,
    "pid": 12345
  }
}
```

### GET /api/sessions

获取所有已记录的 session 列表（包括已完成的 session）。

**Response** `200 OK`:

```json
[
  {
    "sessionId": "<uuid>",
    "source": "claude-code",
    "cwd": "/home/user/project",
    "projectName": "project",
    "status": "completed",
    "startedAt": 1700000000000,
    "updatedAt": 1700000000100,
    "completedAt": 1700000000100,
    "lastMessage": "Task completed.",
    "lastToolName": "Bash",
    "transcriptPath": "/tmp/transcript.json",
    "needsAttention": false,
    "pid": 12345
  }
]
```

### GET /api/health

健康检查端点。

**Response** `200 OK`:

```json
{ "status": "ok" }
```

## 错误码

| 状态码 | 含义 | 可能原因 |
|--------|------|----------|
| `201` | 事件已成功接收并处理 | — |
| `400` | JSON 格式错误或缺失必要字段 | 检查 JSON 语法和字段名 |
| `404` | 路由不存在 | 检查 URL 路径和方法 |
| `500` | 服务器内部错误 | 数据库锁冲突或写入失败 |

## 事件类型映射

| `hook_event_name` | `eventType` | `status` | 来源 |
|-------------------|-------------|----------|------|
| `SessionStart` | `session_start` | `starting` | CC / Codex |
| `PreToolUse` | `pre_tool_use` | `tool_running` | CC / Codex |
| `PostToolUse` | `post_tool_use` | `running` | CC / Codex |
| `PostToolUseFailure` | `failure` | `failed` | CC only |
| `Notification` (permission_prompt) | `permission_request` | `waiting_permission` | CC only |
| `Notification` (idle_prompt) | `notification` | `waiting_input` | CC only |
| `PermissionRequest` | `permission_request` | `waiting_permission` | Codex only |
| `Stop` / `SubagentStop` | `stop` | `completed` | CC / Codex |
| `UserPromptSubmit` | `notification` | `running` | CC / Codex |

## 示例：curl 测试

```powershell
# 健康检查
curl http://127.0.0.1:17878/api/health

# 模拟 SessionStart
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/demo","hook_event_name":"SessionStart","process_pid":4}'

# 模拟 PreToolUse
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/demo","hook_event_name":"PreToolUse","tool_name":"Bash"}'

# 模拟 Stop
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/demo","hook_event_name":"Stop"}'

# 查看全部 sessions
curl http://127.0.0.1:17878/api/sessions
```
