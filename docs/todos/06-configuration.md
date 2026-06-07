# ~~TODO: 可配置化~~ ✅ 全部完成

> 状态：**全部完成** — 4/4 项已实现（验证日期: 2026-06-07）
>
> 消除硬编码，统一从配置文件读取，环境变量作为次级覆盖

---

## 架构

新增 `config.rs` 模块，定义 `AgentPulseConfig` 结构体：

- **主配置源**: `{app_data_dir}/config.json`（首次启动自动生成默认文件）
- **次级覆盖**: 环境变量（方便 CI/容器场景）
- **前端获取**: 通过 Tauri command `get_config` 读取

### 配置文件示例 (`config.json`)

```json
{
  "port": 17878,
  "checkIntervalSecs": 5,
  "python": null,
  "pollIntervalMs": 2000
}
```

- `python: null` → 自动探测（`python3` → `python`）
- `python: "python3.12"` → 使用指定解释器

---

## 6.1 HTTP 端口硬编码 ✅

**已完成**: `lib.rs` 从 `config.port` 读取端口，默认 17878。Python 侧 `monitor_hook.py` 已支持 `AGENTPULSE_URL`。

**配置方式**: 编辑 `config.json` 中的 `port` 字段，或设置 `AGENTPULSE_PORT` 环境变量。

**文件**: [config.rs](apps/desktop/src-tauri/src/config.rs), [lib.rs:148](apps/desktop/src-tauri/src/lib.rs#L148)

---

## 6.2 Process Checker 轮询间隔硬编码 ✅

**已完成**: `process_checker::start()` 接受 `interval_secs` 参数，由 `config.check_interval_secs` 传入。

**配置方式**: 编辑 `config.json` 中的 `checkIntervalSecs` 字段，或设置 `AGENTPULSE_CHECK_INTERVAL`。

**文件**: [config.rs](apps/desktop/src-tauri/src/config.rs), [process_checker.rs:28](apps/desktop/src-tauri/src/process_checker.rs#L28)

---

## 6.3 Python 解释器名假设 ✅

**已完成**: `hooks::resolve_python(hint)` 函数：
1. 配置文件 `python` 字段优先
2. `AGENTPULSE_PYTHON` 环境变量覆盖
3. 自动探测 `python3` → `python`

`build_hook_configs` 和 `ensure_hooks_installed` 接受 `python` 参数，不再内部硬编码。

**配置方式**: 编辑 `config.json` 中的 `python` 字段，或设置 `AGENTPULSE_PYTHON`。

**文件**: [hooks.rs:97-115](apps/desktop/src-tauri/src/hooks.rs#L97-L115)

---

## 6.4 前端轮询间隔硬编码 ✅

**已完成**: `FloatingPanel.vue` 在 `onMounted` 时通过 IPC 调用 `get_config()` 获取 `pollIntervalMs`，替代硬编码。

**配置方式**: 编辑 `config.json` 中的 `pollIntervalMs` 字段，或设置 `AGENTPULSE_POLL_INTERVAL`。

**文件**: [FloatingPanel.vue:22-24](apps/desktop/src/components/FloatingPanel.vue#L22-L24), [ipc.ts:19-26](apps/desktop/src/utils/ipc.ts#L19-L26)

---

## 配置速查

| 配置文件字段 | 环境变量 | 默认值 | 说明 |
|-------------|----------|--------|------|
| `port` | `AGENTPULSE_PORT` | `17878` | HTTP 事件服务器端口 |
| `checkIntervalSecs` | `AGENTPULSE_CHECK_INTERVAL` | `5` | 进程存活检查间隔（秒） |
| `python` | `AGENTPULSE_PYTHON` | 自动检测 | Python 解释器路径 |
| `pollIntervalMs` | `AGENTPULSE_POLL_INTERVAL` | `2000` | 前端轮询间隔（毫秒） |

### 优先级

`config.json` > 内置默认值，环境变量 > `config.json`。

### 涉及文件

| 文件 | 变更 |
|------|------|
| [config.rs](apps/desktop/src-tauri/src/config.rs) | **新增** — 配置加载/保存/默认值 |
| [lib.rs](apps/desktop/src-tauri/src/lib.rs) | 加载 config，传递到 process_checker / hooks / AppState |
| [process_checker.rs](apps/desktop/src-tauri/src/process_checker.rs) | `start()` 接受 `interval_secs` 参数 |
| [hooks.rs](apps/desktop/src-tauri/src/hooks.rs) | `resolve_python()` 接受 hint，修改函数签名 |
| [commands.rs](apps/desktop/src-tauri/src/commands.rs) | AppState 加 config，新增 `get_config` 命令 |
| [ipc.ts](apps/desktop/src/utils/ipc.ts) | 新增 `getConfig()` |
| [FloatingPanel.vue](apps/desktop/src/components/FloatingPanel.vue) | 从后端获取 pollIntervalMs |
