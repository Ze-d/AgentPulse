# TODO: 高优先级修复

> 生产就绪的阻断性问题：安全漏洞、数据丢失、可触发 panic

---

## 1.1 CSP 安全策略缺失 🔴

**问题**: `tauri.conf.json` 中 `"csp": null`，前端无任何脚本注入防护。若 hook 事件中的 `message`、`cwd` 等字段被恶意构造并渲染到 WebView，存在 XSS 风险。

**文件**: [apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json)

**建议**: 定义严格的 CSP，至少设为 `"default-src 'self'; style-src 'self' 'unsafe-inline'"`。

---

## 1.2 DB 反序列化 panic 🔴

**问题**: `db.rs:87,91,95` 中 `deserialize_agent_source`、`deserialize_agent_status`、`deserialize_event_type` 使用了 `unwrap()`。若数据库中出现异常值（如网络传入的错误枚举），将直接 panic。

**文件**: [apps/desktop/src-tauri/src/db.rs](../../apps/desktop/src-tauri/src/db.rs)

**建议**: 改为返回 `Result`，或将 `unwrap()` 替换为 `unwrap_or_else(|e| panic!("corrupt DB value: {e}"))` 以提供有用的错误信息。

---

## 1.3 Event Server 锁强制 unwrap 🔴

**问题**: `event_server.rs:225` 中 `event_server.db.lock().unwrap()` — 若任意持有 Mutex 的线程 panic，锁变为 poisoned，所有后续 `.lock().unwrap()` 连锁崩溃。

**文件**: [apps/desktop/src-tauri/src/event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs)

**建议**: 参照 `commands.rs` 和 `process_checker.rs` 的作法，使用 `match` + `log::error!` + 返回 500，优雅处理 poisoning。

---

## 1.4 process_pid 数据丢失 🟡

**问题**: `db.rs:229` 中 `get_events_for_session` 将 `process_pid` 硬编码为 `None`，且 events 表缺少 `process_pid` 列。event 的 PID 在写入后即丢失，前端永远看不到。

**文件**: [apps/desktop/src-tauri/src/db.rs](../../apps/desktop/src-tauri/src/db.rs)

**建议**: 在 events 表添加 `process_pid INTEGER` 列，读出时正确映射。

---

## 1.5 Event Server 无服务端错误日志 🟡

**问题**: `event_server.rs` 接收 POST body 处理失败时，错误仅返回给 HTTP 客户端，不在服务端记录。Hook 调试困难。

**文件**: [apps/desktop/src-tauri/src/event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs)

**建议**: 在 `handle_event` 失败分支添加 `log::error!("handle event: {e}")`。

---

## 1.6 Event Server 线程无法优雅关闭 🟡

**问题**: HTTP server 线程脱离（detach）运行，无 shutdown signal。仅能通过杀死进程终止。

**文件**: [apps/desktop/src-tauri/src/event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs)

**建议**: 引入 `AtomicBool` shutdown flag，在 `for` 循环中检测，或在 Drop 时关闭 server socket。

---

## 1.7 `list_active_sessions` 已不存在 🟡

**问题**: 之前的提交将 `get_sessions` 改为调用 `list_all_sessions`，但 db.rs 中可能残留未清理的旧函数引用。需确认 db.rs 中无 dead code。

**文件**: [apps/desktop/src-tauri/src/db.rs](../../apps/desktop/src-tauri/src/db.rs)
