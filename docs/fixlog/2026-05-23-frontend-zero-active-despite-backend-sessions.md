# BugFix: Frontend 显示 "0 active" 但后端有活跃 session

**日期:** 2026-05-23
**严重程度:** 中（功能可用但 UI 不更新）
**状态:** 已修复

## 现象

AgentPulse 浮窗显示 "0 active, no active session"，但通过 `curl http://127.0.0.1:17878/api/sessions` 能查到活跃的 Claude Code session。

## 排查过程

1. **验证 HTTP API** — `/api/sessions` 返回 1 个 session，状态 `tool_running`，确认事件服务器正常接收和存储事件
2. **验证 hooks** — `~/.claude/settings.json` 中 6 个 hook 事件已正确注册，monitor_hook.py 正常转发事件
3. **验证数据库共享** — 审查 `lib.rs`，确认 `Arc<Mutex<Database>>` 正确克隆给事件服务器线程和 Tauri state
4. **添加调试日志** — 在 Rust `get_sessions` 命令和前端 `fetchSessions` 中加日志
5. **发现关键证据** — Rust 日志显示 `get_sessions called, returning N sessions` 每 2 秒被调用，证明前端轮询正常、后端返回正常

## 根因分析

**结论：时序问题导致的数据竞争（不是代码 bug）。**

```
时间线:
T1: 启动 AgentPulse → 内存数据库为空
T2: 安装 hooks 到 settings.json
T3: CC session 触发 PreToolUse 事件 → 写入 DB
T4: 前端轮询 → get_sessions 返回 1 session
T5: Tauri 进程异常退出（后台任务 exit code 0）→ 内存数据库丢失
T6: 用户查看浮窗 → 前端无法连接 Tauri 后端 → 显示缓存状态 "0 active"
```

**直接原因:** 第一次 `tauri dev` 启动时，Vite dev server 和 Rust 后端进程分离。前端无法正常与已退出的 Rust 后端通信，显示的是连接断开前缓存的状态。

**本质原因:** 内存数据库 (`Database::new_in_memory()`) 随进程退出而丢失，缺乏持久化能力使得问题难以事后复现。

## 修复

1. **清理残余进程** — 终止所有 node.exe 和 desktop.exe 进程
2. **干净重启** — 重新运行 `npm run tauri dev`，确保前后端进程完整启动
3. **保留调试日志** — 前端 `fetchSessions` 增加 `console.debug` 日志，便于未来排查
4. **验证** — 重启后 hooks 自动向新 AgentPulse 实例推送事件，浮窗正确显示 session

## 验证方式

```powershell
# 1. 确认 AgentPulse 运行中
curl http://127.0.0.1:17878/api/health
# → {"status":"ok"}

# 2. 确认 hooks 生效（在 CC session 中执行任意工具调用）
# 查看数据库中的 session
curl http://127.0.0.1:17878/api/sessions
# → [{"sessionId":"...","status":"tool_running","projectName":"AgentPulse"}]

# 3. 浮窗应显示 "N active" 和对应 session 卡片
```

## 后续建议

- [ ] 将数据库从 `new_in_memory()` 改为文件持久化（`Connection::open("agentpulse.db")`），避免进程退出丢失数据
- [ ] 添加进程守护 / 自动重启逻辑
- [ ] 前端检测 Tauri IPC 连接断开时显示明确提示而非空状态
