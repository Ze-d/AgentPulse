# TODO: 可配置化

> 消除硬编码，支持环境变量或配置文件

---

## 6.1 HTTP 端口硬编码 🟡

**问题**: `127.0.0.1:17878` 硬编码在 `lib.rs` 和 `monitor_hook.py` 中。与其他软件端口冲突时无法更改。

**文件**:
- [apps/desktop/src-tauri/src/lib.rs](../../apps/desktop/src-tauri/src/lib.rs)
- [adapters/claude-code/monitor_hook.py](../../adapters/claude-code/monitor_hook.py)

**建议**:
- Rust: 通过环境变量 `AGENTPULSE_PORT` 读取，默认 17878
- Python: 已支持 `AGENTPULSE_URL` 环境变量（部分覆盖），但需确保 Rust 侧同步

---

## 6.2 Process Checker 轮询间隔硬编码 🟢

**问题**: `Duration::from_secs(5)` 硬编码在 `process_checker.rs`。

**文件**: [apps/desktop/src-tauri/src/process_checker.rs](../../apps/desktop/src-tauri/src/process_checker.rs)

**建议**: 通过环境变量 `AGENTPULSE_CHECK_INTERVAL` 读取，默认 5。

---

## 6.3 Python 解释器名假设 🟢

**问题**: `hooks.rs` 使用 `"python"` 作为执行命令。部分 Linux 发行版中 Python 3 为 `python3`。

**文件**: [apps/desktop/src-tauri/src/hooks.rs](../../apps/desktop/src-tauri/src/hooks.rs)

**建议**: 先尝试 `python3`，回退到 `python`。

---

## 6.4 前端轮询间隔硬编码 🟢

**问题**: `startPolling(2000)` 的 2 秒间隔在组件中硬编码。

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)

**建议**: 从配置文件或环境变量读取，允许用户自定义。
