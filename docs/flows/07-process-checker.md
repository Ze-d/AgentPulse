# 7. 进程存活检测流程

## 涉及文件

- [process_checker.rs](../../apps/desktop/src-tauri/src/process_checker.rs) — Rust 端进程检测
- [monitor_hook.py](../../adapters/claude-code/monitor_hook.py) — Python 端 PID 获取

## 概述

进程存活检测分为两个阶段：
1. **PID 获取** — `monitor_hook.py` 通过 Windows API 向上遍历进程树获取 CC node.exe 的真实 PID
2. **PID 存活检测** — `process_checker.rs` 后台线程定期检查活跃 session 的 PID 是否还存在（间隔可配置，默认 5 秒）

## 阶段 1: PID 获取（Python 端）

详见 [03-event-capture.md](03-event-capture.md) 中的"PID 探测机制详解"章节。

流程概要：
```
monitor_hook.py 被 CC hook 触发
  │
  └─→ _walk_process_tree_to_cc()
        ├─→ 非 Windows → os.getppid()
        └─→ Windows:
              ├─→ _snapshot_processes()           // CreateToolhelp32Snapshot
              └─→ 从当前 PID 向上遍历进程树
                    ├─→ 跳过 shell 进程 (cmd.exe, powershell.exe, ...)
                    └─→ 找到第一个非 shell 父进程 → 返回其 PID
```

## 阶段 2: 进程存活检测（Rust 端）

```
process_checker::start(db: Arc<Mutex<Database>>, interval_secs: u64)
  │
  └─→ thread::spawn {
        loop {
          sleep(interval_secs 秒)   // 来自配置，默认 5
          │
          ├─→ 1. db.lock() → db.list_sessions_with_pid()
          │     │
          │     │  SQL:
          │     │    SELECT * FROM sessions WHERE pid IS NOT NULL
          │     │    ORDER BY updated_at DESC
          │     │
          │     └─→ 无 PID 的 session → 跳过
          │
          ├─→ 2. system.refresh_processes(ProcessesToUpdate::All)
          │     │
          │     └─→ sysinfo 刷新系统进程列表
          │
          └─→ 3. for each session:
                │
                ├─→ pid = session.pid
                │
                ├─→ is_active_status(session.status)?
                │     │
                │     ├─→ 活跃状态 (Starting, Running, ToolRunning,
                │     │             WaitingInput, WaitingPermission):
                │     │     │
                │     │     └─→ sysinfo::System::process(Pid)
                │     │           │
                │     │           ├─→ Some(_) → 进程存活，跳过
                │     │           │
                │     │           └─→ None → 进程已死
                │     │                 │
                │     │                 ├─→ log::info("PID {} gone, removing session {}")
                │     │                 └─→ db.delete_session(session_id)
                │     │                       ├─→ DELETE FROM events WHERE session_id = ?
                │     │                       └─→ DELETE FROM sessions WHERE session_id = ?
                │     │
                │     └─→ 终端状态 (Completed, Failed, Unknown):
                │           └─→ 跳过！保留展示
                │
                └─→ 继续下一个 session
        }  // 循环
      }
```

## is_active_status 过滤逻辑

```rust
fn is_active_status(status: &AgentStatus) -> bool {
    !matches!(
        status,
        AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Unknown
    )
}
```

| 状态 | is_active_status | 行为 |
|------|-----------------|------|
| `Starting` | ✅ true | 检测 PID，死亡则删除 |
| `Running` | ✅ true | 检测 PID，死亡则删除 |
| `ToolRunning` | ✅ true | 检测 PID，死亡则删除 |
| `WaitingInput` | ✅ true | 检测 PID，死亡则删除 |
| `WaitingPermission` | ✅ true | 检测 PID，死亡则删除 |
| `Completed` | ❌ false | 跳过，保留展示 |
| `Failed` | ❌ false | 跳过，保留展示 |
| `Unknown` | ❌ false | 跳过，保留展示 |

## 清理时机对比

| 清理方式 | 触发条件 | 清理对象 |
|----------|----------|----------|
| Process Checker | PID 进程死亡 + 非终端状态 | 活跃 session |
| 应用重启 | 应用退出 | 所有 session（内存数据库） |
| Retention Cleanup | 定期调用 `cleanup_old_sessions()` | 旧的 Completed/Failed session |

## 边界情况

### 情况 1: Session 没有 PID

- 原因: SessionStart 事件丢失，后续事件也没有获取到 PID
- 行为: `list_sessions_with_pid()` 不会返回该 session
- 结果: 永远不会被 process_checker 自动清理
- 解决: 重启应用（内存数据库清空）

### 情况 2: CC 进程崩溃

- CC 进程异常退出，没有发送 `Stop` 事件
- Session 状态停留在 `Running` 或 `ToolRunning`
- Process Checker 检测到 PID 死亡 → 删除 session
- **延迟**: 最多 5 秒（检测间隔）

### 情况 3: CC 正常退出

- CC 发送 `Stop` 事件 → session 状态变为 `Completed`
- Process Checker 看到 `Completed` → 跳过删除
- 卡片保留 `[done]` 状态
- 之后 CC 进程退出，PID 死亡
- 但 session 已是 `Completed` → 不会被删除
- 下次重启应用时清空

### 情况 4: 同一 PID 被复用

- 操作系统可能回收 PID
- 如果新进程恰好使用了旧 session 的 PID
- Process Checker 会看到 PID 存活 → 不会误删
- 这是可接受的行为（宁可保留多于删除）

### 情况 5: 锁中毒

```rust
let d = match db.lock() {
    Ok(d) => d,
    Err(_) => continue,  // 跳过本轮，5 秒后重试
};
```

- 如果数据库 Mutex 中毒，跳过本轮检测
- 下一轮（5 秒后）重试

## 性能考虑

- 5 秒间隔：平衡及时性和 CPU 开销
- `system.refresh_processes(All)`：刷新全部进程，因为只需要检查少量 PID，开销可接受
- 数据库锁持有时间短：查询 → 检查 → 删除，立即释放锁
