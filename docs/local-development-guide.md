# AgentPulse 本地开发指南

## 环境要求

| 工具 | 版本 | 用途 |
|------|------|------|
| Node.js | >= 18 | 前端构建 |
| Rust | >= 1.70 (MSVC toolchain) | Tauri 后端 |
| Python | >= 3.8 | Hook 适配器脚本 |
| Git | 任意 | 版本控制 |

### Windows 特别说明

需要 MSVC 工具链（而非 GNU），否则 `rusqlite` 的 bundled 编译会出错。验证：

```powershell
rustup show
# 应显示: stable-x86_64-pc-windows-msvc
```

如果不是 MSVC：
```powershell
rustup default stable-x86_64-pc-windows-msvc
```

---

## 项目结构速览

```
AgentPulse/
├── apps/desktop/                  # Tauri 应用主目录
│   ├── src/                       # Vue 3 前端
│   │   ├── components/            # .vue 组件
│   │   ├── stores/                # Pinia 状态管理
│   │   ├── types/                 # TypeScript 类型
│   │   └── assets/                # CSS (Tailwind + Catppuccin)
│   ├── src-tauri/                 # Rust 后端
│   │   ├── src/
│   │   │   ├── lib.rs             # 共享类型
│   │   │   ├── db.rs              # SQLite 数据库
│   │   │   ├── state_machine.rs   # 状态机
│   │   │   ├── event_server.rs    # HTTP 事件服务器 :17878
│   │   │   ├── commands.rs        # Tauri 命令（前后端桥接）
│   │   │   ├── window.rs          # 悬浮窗创建
│   │   │   ├── tray.rs            # 系统托盘
│   │   │   └── main.rs            # 入口点
│   │   └── tests/                 # Rust 集成测试
│   ├── package.json
│   └── tauri.conf.json
├── adapters/claude-code/          # Claude Code hooks 适配器
│   ├── monitor_hook.py            # 事件转发脚本
│   └── install_hooks.py           # 一键安装/卸载 hooks
└── tests/integration/             # E2E 测试
```

---

## 一、首次安装

```powershell
# 进入 Tauri 应用目录
cd apps/desktop

# 安装前端依赖
npm install

# 验证 Rust 编译（首次会下载依赖，需要几分钟）
cd src-tauri
cargo check
cd ..
```

---

## 二、本地调试运行

### 2.1 仅前端（Vite dev server）

不需要 Rust 后端，仅调试 UI：

```powershell
cd apps/desktop
npm run dev
```

浏览器打开 `http://localhost:1420`。此时：
- 可以看到 UI 布局和样式
- Tauri API 调用会失败（没有 Rust 后端），store 会显示错误信息

### 2.2 完整 Tauri 应用（推荐调试方式）

```powershell
cd apps/desktop
npm run tauri dev
```

这会同时启动：
1. Vite dev server (前端热更新，端口 1420)
2. Rust 后端（事件服务器端口 17878）
3. Tauri 悬浮窗

**调试技巧：**
- 前端代码修改后自动热更新
- Rust 代码修改后自动重编译（`cargo watch` 模式）
- 悬浮窗右键 → Inspect 可打开 Chrome DevTools

### 2.3 模拟 Hook 事件

在 Tauri 应用运行期间，另一个终端手动发送事件测试：

```powershell
# 健康检查
curl http://127.0.0.1:17878/api/health

# 模拟 SessionStart
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/test-project","hook_event_name":"SessionStart","transcript_path":"D:/tmp/transcript.json"}'

# 模拟 PreToolUse
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/test-project","hook_event_name":"PreToolUse","tool_name":"Bash"}'

# 模拟 PostToolUse
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/test-project","hook_event_name":"PostToolUse","tool_name":"Bash"}'

# 模拟 Notification（等待权限）
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/test-project","hook_event_name":"Notification","notification_type":"permission_prompt","message":"需要批准执行命令"}'

# 模拟 Stop（任务完成）
curl -X POST http://127.0.0.1:17878/api/events `
  -H "Content-Type: application/json" `
  -d '{"session_id":"test-001","cwd":"D:/projects/test-project","hook_event_name":"Stop","last_assistant_message":"任务已完成"}'

# 查看活跃 sessions
curl http://127.0.0.1:17878/api/sessions
```

浮窗应该实时显示 session 状态变化。

### 2.4 测试 Python Hook 适配器

```powershell
# 模拟 Claude Code 通过 stdin 传入 hook 数据
echo '{"session_id":"test-002","cwd":"D:/tmp","hook_event_name":"SessionStart"}' | python adapters/claude-code/monitor_hook.py
```

---

## 三、运行测试

### 3.1 Rust 测试

```powershell
cd apps/desktop/src-tauri
cargo test
```

预期：25 tests passed（7 db 单元 + 3 db 集成 + 5 event_server + 7 state_machine + 3 types）

### 3.2 前端类型检查

```powershell
cd apps/desktop
npx vue-tsc --noEmit
```

预期：零错误。

### 3.3 前端构建验证

```powershell
cd apps/desktop
npm run build
```

预期：`vite build` 成功，输出到 `dist/`。

### 3.4 E2E 测试

需要先启动 Tauri 应用（`npm run tauri dev`），然后在另一个终端：

```powershell
python tests/integration/test_e2e.py
```

如果服务器未运行，测试会优雅跳过（不会报错）。

---

## 四、打包为应用

### 4.1 生成安装包

```powershell
cd apps/desktop
npm run tauri build
```

产物位置：
- **Windows**: `apps/desktop/src-tauri/target/release/bundle/`
  - `.msi` — Windows 安装包
  - `.exe` — 独立可执行文件
  - `.msi` + `.exe` 均在 `bundle/msi/` 和 `bundle/nsis/` 下

### 4.2 仅编译 Rust 二进制（调试用）

```powershell
cd apps/desktop/src-tauri
cargo build --release
```

产物：`target/release/agentpulse.exe`（注意：需要前端 dist 目录存在才能正常运行）

### 4.3 打包配置参考

`tauri.conf.json` 中的 bundle 配置：

```json
{
  "bundle": {
    "active": true,
    "targets": "all",        // msi + nsis on Windows
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

**更换图标**：替换 `apps/desktop/src-tauri/icons/` 下的图标文件即可。

---

## 五、接入 Claude Code

### 5.1 安装 Hooks

```powershell
# 预览将要执行的操作
python adapters/claude-code/install_hooks.py --dry-run

# 安装 hooks
python adapters/claude-code/install_hooks.py

# 查看安装状态
python adapters/claude-code/install_hooks.py --status
```

这会在 `~/.claude/settings.json` 中写入 6 个 hook 事件的配置：

- SessionStart
- PreToolUse
- PostToolUse
- PostToolUseFailure
- Notification
- Stop

### 5.2 验证 Hooks 是否生效

```powershell
# 方式 1: 使用 --status
python adapters/claude-code/install_hooks.py --status

# 方式 2: 直接查看配置
python -c "import json; from pathlib import Path; s = json.loads(Path.home().joinpath('.claude', 'settings.json').read_text()); print(json.dumps(s.get('hooks', {}), indent=2))"
```

应能看到 `"hooks"` 字段包含 6 个事件配置，每个指向 `monitor_hook.py`。

### 5.3 卸载 Hooks

```powershell
python adapters/claude-code/install_hooks.py --remove
```

### 5.4 调试 Hook 事件

```powershell
# 测试 monitor_hook.py 是否正确解析 stdin（不发送到服务器）
echo '{"session_id":"debug-1","cwd":"/tmp/test","hook_event_name":"SessionStart"}' | python adapters/claude-code/monitor_hook.py --test

# 开启详细日志
$env:AGENTPULSE_LOG_LEVEL = "DEBUG"
echo '{"session_id":"debug-1","cwd":"/tmp/test","hook_event_name":"SessionStart"}' | python adapters/claude-code/monitor_hook.py

# 指定事件服务器地址（如果非默认端口）
$env:AGENTPULSE_URL = "http://127.0.0.1:9999/api/events"
echo '...' | python adapters/claude-code/monitor_hook.py
```

### 5.5 使用流程

1. 启动 AgentPulse：`npm run tauri dev`（或运行打包后的 exe）
2. 正常使用 Claude Code
3. AgentPulse 浮窗自动显示 CC session 状态

### 5.6 安全说明

- 安装和卸载操作会自动在修改前备份 `settings.json` → `settings.json.bak`
- 重复安装不会覆盖已有配置（幂等操作），使用 `--force` 强制覆盖
- 卸载时只移除 AgentPulse 的 6 个 hook 事件，保留其他自定义 hooks

---

## 六、常见问题

### Q: `cargo check` 报 `rusqlite` 编译错误

```
error: linking with `link.exe` failed: exit code: 1181
```

→ 切换到 MSVC 工具链：
```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### Q: `npm run tauri dev` 窗口不显示

→ 检查 `tauri.conf.json` 中 `"visible": true`，确保 `"alwaysOnTop": true`。

### Q: 前端能打开但 Rust 调用报错

→ 确认 Tauri 是通过 `npm run tauri dev` 启动的（不是 `npm run dev`）。`npm run dev` 只启 Vite，没有 Rust 后端。

### Q: curl POST 返回 400

→ 检查 JSON 格式，Windows PowerShell 的 curl 需要用反引号 ` 续行，且 JSON 内部只能用双引号。

### Q: Python 脚本报 `python: command not found`

→ 用 `python` 或 `python3`，取决于你的 Python 安装方式。Windows 上通常是 `python`。

### Q: 如何看到 Tauri 的日志输出

→ Rust 后端使用 `env_logger`，启动前设置环境变量：
```powershell
$env:RUST_LOG = "debug"
npm run tauri dev
```
