# 3. 事件捕获与转发流程

## 涉及文件

- [monitor_hook.py](../../adapters/claude-code/monitor_hook.py) — 核心适配器脚本

## 概述

当 Claude Code 触发 hook 事件时，CC 通过 stdin 将 JSON 数据传给 `monitor_hook.py`，脚本负责：
1. 读取 stdin 中的 JSON
2. 通过 Windows 进程树 API 获取 CC 真实 PID
3. 将增强后的事件通过 HTTP POST 发送到 AgentPulse 服务器

## 完整流程

```
Claude Code 触发 hook 事件 (如 SessionStart, PreToolUse, ...)
  │
  │  CC 读取 ~/.claude/settings.json 中的 hooks 配置
  │
  └─→ 执行: python "C:\...\monitor_hook.py"
        │
        │  CC 通过 stdin 传入 hook JSON 数据:
        │  {
        │    "hook_event_name": "SessionStart",
        │    "session_id": "abc-123",
        │    "cwd": "D:/projects/my-app",
        │    "transcript_path": "/tmp/transcript.json",
        │    "message": "...",
        │    "tool_name": "Bash",
        │    ...
        │  }
        │
        └─→ main()
              │
              ├─→ 1. read_stdin()
              │     ├─→ sys.stdin.read()                           // 读取全部内容
              │     ├─→ .strip() → 空 → 返回 None，exit(0)        // 无输入，正常退出
              │     └─→ json.loads() → 成功 → 返回 dict
              │                   → 失败 → logger.error + exit(1)  // 解析错误
              │
              ├─→ 2. _walk_process_tree_to_cc()
              │     │
              │     │  Windows (win32):
              │     │  │
              │     │  ├─→ _snapshot_processes()
              │     │  │     ├─→ CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
              │     │  │     ├─→ Process32First/Next 遍历所有进程
              │     │  │     └─→ 返回 (pid→parent_pid, pid→name) 两个字典
              │     │  │
              │     │  └─→ 从当前 PID 向上遍历（最多 5 层）:
              │     │        cur = os.getpid()
              │     │        for _ in range(5):
              │     │            parent = pid_to_parent[cur]
              │     │            name = pid_to_name[parent]
              │     │            if name NOT IN {cmd.exe, powershell.exe, pwsh.exe,
              │     │                            sh.exe, bash.exe, conhost.exe}:
              │     │                return parent   // ← 找到了！这是 CC 的 node.exe
              │     │            cur = parent        // 跳过 shell，继续向上
              │     │        return os.getppid()     // 兜底
              │     │
              │     │  非 Windows:
              │     │    return os.getppid()         // 直接返回父进程 PID
              │     │
              │     └─→ hook_data["process_pid"] = <CC PID>
              │
              ├─→ 3. send_event(AGENTPULSE_URL, hook_data, TIMEOUT)
              │     │
              │     │  URL: http://127.0.0.1:17878/api/events
              │     │  Content-Type: application/json
              │     │
              │     └─→ 重试循环 (最多 3 次):
              │           for attempt in 1..=3:
              │             try:
              │               resp = urllib.request.urlopen(req, timeout=5s)
              │               status == 201 → logger.info + return status
              │               status != 201 → logger.warning + return status
              │             catch HTTPError → return e.code
              │             catch URLError/OSError:
              │               if attempt < 3: sleep(1s), retry
              │               else: logger.error + return -1
              │
              └─→ exit(0)  [成功] 或 exit(1)  [失败]
```

## PID 探测机制详解

### 为什么需要进程树遍历？

Claude Code 启动 hook 时的进程链：

```
node.exe (Claude Code, PID=1234)     ← 我们需要这个 PID
  └─→ cmd.exe / powershell.exe       ← CC 通过 shell 启动 hook
        └─→ python.exe                ← monitor_hook.py 运行在这里
```

`os.getppid()` 返回的是 shell 的 PID（如 `cmd.exe`，PID=5678），但 shell 会在 hook 脚本运行时已退出。我们需要向上跳过 shell 中间层，找到真正的 `node.exe` PID。

### 遍历策略

```
进程树:
  PID=1234  node.exe        ← 目标：不是 shell，是真正的 CC 进程
  PID=5678  cmd.exe          ← 跳过（在 _SHELL_NAMES 中）
  PID=9012  python.exe       ← 当前进程（起点）

遍历:
  cur=9012 → parent=5678 (cmd.exe, 跳过) → cur=5678
  cur=5678 → parent=1234 (node.exe, 不在 _SHELL_NAMES) → 返回 1234 ✓
```

### 安全限制

- 最多向上遍历 5 层，防止死循环
- 异常情况兜底返回 `os.getppid()`
- 非 Windows 平台直接返回 `os.getppid()`

## HTTP 转发重试策略

| 场景 | 行为 |
|------|------|
| HTTP 201 | 成功，返回 status |
| HTTP 4xx/5xx | 立即返回，不重试（服务器明确拒绝了请求） |
| URLError / OSError | 重试最多 3 次，间隔 1s |
| 3 次重试后仍失败 | 返回 -1，exit(1) |

### 环境变量配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `AGENTPULSE_URL` | `http://127.0.0.1:17878/api/events` | 服务器地址 |
| `AGENTPULSE_TIMEOUT` | `5` | HTTP 请求超时（秒） |
| `AGENTPULSE_LOG_LEVEL` | `INFO` | 日志级别 (DEBUG/INFO/WARNING/ERROR) |

## 测试模式

```powershell
echo '{"hook_event_name":"SessionStart","session_id":"test-001",...}' | python monitor_hook.py --test
```

`--test` 模式不会 POST 到服务器，而是将处理后的 JSON 打印到 stdout，方便调试。
