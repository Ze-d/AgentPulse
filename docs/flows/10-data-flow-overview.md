# 10. 整体数据流总览

## 端到端数据流（完整链路）

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Claude Code 终端                                │
│                                                                          │
│  用户输入 "帮我写个测试"                                                  │
│  CC 触发 hook 事件，读取 ~/.claude/settings.json                         │
│                                                                          │
│  每次事件触发时:                                                          │
│    python "C:\...\monitor_hook.py" <<< '{ "hook_event_name": "...",  }' │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ stdin (JSON)
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    monitor_hook.py (Python)                               │
│                                                                          │
│  1. sys.stdin.read() → json.loads()                                     │
│  2. _walk_process_tree_to_cc() → 获取 CC node.exe 真实 PID              │
│     ├─ CreateToolhelp32Snapshot → 枚举所有进程                          │
│     └─ 向上遍历父进程，跳过 shell → 找到 node.exe                        │
│  3. hook_data["process_pid"] = CC_PID                                  │
│  4. HTTP POST http://127.0.0.1:{port}/api/events                       │
│     └─ 最多 3 次重试，间隔 1s                                           │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ HTTP POST /api/events
                               │ Content-Type: application/json
                               │ Body: { hook_event_name, session_id,
                               │         cwd, tool_name, process_pid, ... }
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                  event_server.rs (Rust, 端口可配置，默认 17878)            │
│                                                                          │
│  1. 读取请求体 → serde_json::from_str()                                 │
│  2. normalize_claude_code_event(raw_json)                               │
│     ├─ 提取字段 (hook_event_name → EventType)                           │
│     ├─ 推导 project_name = basename(cwd)                                │
│     └─ 映射 event_type + status                                         │
│                                                                          │
│  3. db.get_session(session_id) — 查找已有 session                       │
│     ├─ 已有 → 合并字段 + 状态机转换                                     │
│     │   └─ state_machine.transition(old_status, event_type)             │
│     └─ 新建 → 创建 AgentSession                                         │
│                                                                          │
│  4. db.upsert_session(&session)  ←→  SQLite (sessions 表)              │
│  5. db.insert_event(&event)      ←→  SQLite (events 表)                │
│                                                                          │
│  6. 返回 HTTP 201 { event: {...}, session: {...} }                      │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
          ┌────────────────────┴────────────────────┐
          │                                         │
          ▼                                         ▼
┌──────────────────────────┐           ┌──────────────────────────┐
│  process_checker.rs      │           │  commands.rs (Tauri IPC) │
│  (独立线程, 可配置间隔)   │           │                          │
│                          │           │  get_sessions()          │
│  list_sessions_with_pid()│           │    → db.list_all()       │
│  → sysinfo 检查 PID 存活 │           │                          │
│  → 死进程 → 删除 session │           │  get_session_detail()    │
└──────────────────────────┘           │  get_session_events()    │
                                       │  get_hook_status_cmd()    │
                                       │  install_hooks_cmd()      │
                                       │  uninstall_hooks_cmd()    │
                                       │  hide_main_window()       │
                                       └───────────┬──────────────┘
                                                   │ Tauri IPC
                                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                  前端 (Vue 3 + Pinia)                                      │
│                                                                          │
│  Pinia sessionStore                                                      │
│  ├─ startPolling(interval) — 可配置间隔（默认 2000ms）                   │
│  │   └─ invoke("get_sessions") → sessions: AgentSession[]               │
│  │                                                                       │
│  └─ getters:                                                             │
│      ├─ activeSessions — 过滤 completed/failed                          │
│      ├─ attentionSessions — needs_attention === true                    │
│      └─ expandedSession — 当前展开的 session                            │
│                                                                          │
│  FloatingPanel.vue                                                       │
│  ├─ onMounted → getConfig() → startPolling(pollIntervalMs)               │
│  ├─ onUnmounted → stopPolling()                                         │
│  ├─ watch(sessions) → adjustWindowSize()                                │
│  │                                                                       │
│  └─ 渲染:                                                                │
│      ├─ 空状态: "$ agentpulse is listening..."                          │
│      └─ 有数据:                                                          │
│          ├─ SessionCard × N (每张 34px)                                  │
│          │   ├─ source-abbr > project-name                              │
│          │   ├─ duration (formatDuration)                                │
│          │   ├─ status-label + status-color                             │
│          │   └─ last-tool-name (如有)                                   │
│          │                                                               │
│          └─ ExpandedDetail (展开时，+120px)                              │
│              ├─ detail-grid (status, duration, cwd, tool, transcript)    │
│              ├─ message-block (最后一条消息)                             │
│              └─ detail-actions (open dir, transcript)                   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 时间线示例

以一次典型的 CC 交互为例：

```
t=0s    用户启动 CC
        CC → SessionStart → monitor_hook.py → POST → EventServer
          → DB: INSERT session { status: Starting, pid: 12345 }
          → UI: 卡片出现，显示 "cc > my-project  0s  starting"

t=1s    用户输入 "写个测试"
        CC → UserPromptSubmit → ... → EventServer
          → state_machine: Starting → Running
          → DB: UPDATE session { status: Running }
          → UI: 卡片更新 "cc > my-project  1s  running"

t=5s    CC 调用 Read 工具
        CC → PreToolUse { tool_name: "Read" } → ... → EventServer
          → state_machine: Running → ToolRunning
          → DB: UPDATE session { status: ToolRunning, last_tool_name: "Read" }
          → UI: 卡片更新 "cc > my-project  5s  tool", 显示 "Read"

t=7s    CC 完成 Read
        CC → PostToolUse → ... → EventServer
          → state_machine: ToolRunning → Running
          → DB: UPDATE session { status: Running }
          → UI: 卡片更新 "cc > my-project  7s  running"

t=10s   CC 需要权限
        CC → Notification { type: permission_prompt } → ... → EventServer
          → state_machine: Running → WaitingPermission
          → DB: UPDATE session { status: WaitingPermission, needs_attention: true }
          → UI: 卡片脉冲高亮 "cc > my-project  10s  permission"

t=15s   用户批准，CC 继续
        CC → PreToolUse { tool_name: "Bash" } → ... → EventServer
          → state_machine: WaitingPermission → ToolRunning
          → UI: 卡片恢复正常

t=30s   CC 完成所有工作
        CC → Stop → ... → EventServer
          → state_machine: Running → Completed
          → DB: UPDATE session { status: Completed, completed_at: now }
          → UI: 卡片显示 "cc > my-project  30s  done" (脉冲)

t=40s   用户关闭 CC 终端
        → CC 进程退出，PID 12345 消失
        → 状态已是 Completed → process_checker 跳过删除
        → 卡片保留

t=45s   用户重启应用
        → 内存数据库清空
        → 卡片消失
```

## 数据存储结构

### sessions 表

| 字段 | 类型 | 说明 |
|------|------|------|
| session_id | TEXT PK | CC 分配的唯一 ID |
| source | TEXT | 来源 (claude-code/codex/gemini/copilot) |
| cwd | TEXT | 工作目录 |
| project_name | TEXT | 项目名 (cwd basename) |
| status | TEXT | 状态 (starting/running/tool_running/waiting_input/waiting_permission/completed/failed) |
| started_at | INTEGER | 创建时间戳 (ms) |
| updated_at | INTEGER | 最后更新时间戳 (ms) |
| completed_at | INTEGER? | 完成时间戳 (ms) |
| last_message | TEXT? | 最后一条消息 |
| last_tool_name | TEXT? | 最后使用的工具名 |
| transcript_path | TEXT? | Transcript 文件路径 |
| needs_attention | INTEGER | 是否需要用户关注 (0/1) |
| pid | INTEGER? | CC 进程 PID |

### events 表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | TEXT PK | UUID v4 |
| source | TEXT | 来源 |
| session_id | TEXT FK | 关联 session |
| cwd | TEXT | 工作目录 |
| project_name | TEXT? | 项目名 |
| event_type | TEXT | 事件类型 |
| status | TEXT | 事件对应的状态 |
| message | TEXT? | 消息内容 |
| tool_name | TEXT? | 工具名 |
| transcript_path | TEXT? | Transcript 路径 |
| created_at | INTEGER | 事件时间戳 (ms) |
| process_pid | INTEGER? | 进程 PID |

## 进程/线程模型

```
主进程 (agentpulse.exe)
  │
  ├─→ 主线程: Tauri 事件循环 + 窗口管理
  │
  ├─→ HTTP Server 线程
  │     └─ tiny_http::Server::incoming_requests() 循环
  │
  ├─→ Process Checker 线程
  │     └─ loop { sleep(Ns); check_pids(); }  // N 来自配置
  │
  └─→ Hook Install 线程 (启动时一次性)
        └─ extract_monitor_script + ensure_hooks_installed

  所有线程共享: Arc<Mutex<Database>>
```

## 锁竞争分析

| 锁持有者 | 持有时间 | 频率 |
|----------|----------|------|
| EventServer (POST /api/events) | ~1ms (查询+写入) | 每次 CC 事件 |
| get_sessions IPC | ~0.5ms (SELECT *) | 可配置，默认每 2 秒 |
| Process Checker | ~0.5ms (查询) / ~1ms (查询+删除) | 可配置，默认每 5 秒 |
| Hook Install (启动时) | 无（操作不同的文件） | 一次 |

锁竞争风险低：所有数据库操作都是简单 CRUD，毫秒级完成。
