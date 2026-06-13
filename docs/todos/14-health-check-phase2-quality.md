# 14 — 项目健康检查：第二阶段 提升工程质量

**状态：** 已完成 ✅
**优先级：** P1/P2 — 消除技术债，补齐测试
**创建日期：** 2026-06-13
**完成日期：** 2026-06-13
**来源：** 项目健康检查报告

---

## 概述

第二阶段聚焦于 **消除代码重复**、**统一双份实现**、**补齐跨模块测试**、**替换废弃依赖**。这些改进直接影响长期维护成本和系统可靠性。

---

## 14.1 🔴 P1 双份 Codex TOML 管理逻辑

**问题**：Codex TOML hook 的安装/卸载/合并逻辑在两个位置独立实现：

| 位置 | 用途 |
|------|------|
| `apps/desktop/src-tauri/src/hooks.rs` L343-459 | Tauri 运行时通过 IPC command 安装 |
| `adapters/hook-adapter/src/installer.rs` L189-367 | CLI 独立工具（`agentpulse-hook install --agent codex`） |

两处逻辑相似但不完全一致。如果 Codex config format 变化，需要改两个地方。

**涉及文件**：
- `apps/desktop/src-tauri/src/hooks.rs`
- `adapters/hook-adapter/src/installer.rs`

**操作**：
1. 评估是否可以删除 adapter 中的 installer 逻辑（如果所有 hook 安装都通过 Tauri 命令完成）
2. 如果不能删除，将 Codex TOML 合并逻辑提取到 adapter 的 lib 模块中，暴露公共函数给 CLI binary 使用
3. Tauri hooks.rs 调用 adapter lib 的公共函数

**验收标准**：
- Codex TOML 格式解析/合并逻辑只有一处
- `cargo test` 在 `hook-adapter/` 和 `src-tauri/` 下均通过
- 两个入口（Tauri 命令 + CLI）行为一致

---

## 14.2 🔴 P1 缺少跨模块集成测试

**问题**：当前测试都是单模块的单元测试，缺少 DB + event_server + state_machine 联合验证。无法自动检测模块间协作 bug（如状态转换与 DB 写入不一致）。

**涉及文件**：
- 新建 `apps/desktop/src-tauri/tests/integration_test.rs`

**操作**：
1. 创建集成测试覆盖完整 session 生命周期：
   - SessionStart → 创建 session（状态 Starting）
   - PreToolUse → 转换到 ToolRunning
   - PostToolUse → 转换到 Running
   - Notification(permission_prompt) → 转换到 WaitingPermission
   - Stop → 转换到 Completed
2. 验证 events 写入和读取顺序
3. 验证 process_pid 在整个生命周期中持久化
4. 验证 `needs_attention` 标志正确切换
5. 验证 process_checker 不误删 Completed session

**验收标准**：
- `cargo test` 包含新的集成测试
- 覆盖至少一个完整的 CC session 生命周期
- 覆盖至少一个完整的 Codex session 生命周期

---

## 14.3 🟡 P1 `tiny_http` 替换为 `axum`

**问题**：`tiny_http = "0.12"` 已长期未维护（最后更新超过 2 年），存在潜在安全风险。且基于同步阻塞 I/O，与 Tauri 2 内置的 tokio runtime 不匹配。

**涉及文件**：
- `apps/desktop/src-tauri/Cargo.toml` — 依赖替换
- `apps/desktop/src-tauri/src/event_server.rs` — HTTP server 重写
- `apps/desktop/src-tauri/src/lib.rs` — 启动方式调整

**推荐方案**：`axum` — Tauri 2 基于 tokio，axum 可复用同一 runtime，生态最活跃。

**操作**：
1. 添加 `axum` + `tokio` 依赖（注意 Tauri 已有的 tokio 版本）
2. 将 `event_server.rs` 中 3 个路由（POST /api/events、GET /api/sessions、GET /api/health）改为 axum router
3. 移除 `std::thread::spawn` 包装，改用 `tokio::spawn`
4. 实现优雅 shutdown（保存 shutdown signal）
5. 更新 `json_response` 辅助函数

**验收标准**：
- 所有 17 个 event_server 测试适配后通过
- `curl` 手动测试 3 个路由行为与之前一致
- `cargo clippy -- -D warnings` 零警告
- 应用退出时 HTTP server 优雅关闭（不再丢弃 shutdown signal）

---

## 14.4 🟡 P2 Release test job 扩展为多平台

**问题**：`.github/workflows/release.yml` 的 `test` job 硬编码 `runs-on: windows-latest`。Linux/macOS 构建前不跑测试。

**涉及文件**：
- `.github/workflows/release.yml`

**操作**：
```yaml
test:
  strategy:
    fail-fast: false
    matrix:
      platform: [windows-latest, ubuntu-latest, macos-latest]
  runs-on: ${{ matrix.platform }}
```
确保 Linux 平台正确安装系统依赖。

**验收标准**：
- 3 平台 test 步骤在下次 tag push 时全部通过

---

## 14.5 🟡 P2 `tailwindcss` 依赖确认与清理

**问题**：`apps/desktop/package.json` 中声明了 `tailwindcss@^4.3.0` 和 `@tailwindcss/vite@^4.3.0`，以及 4 个 optional 的 oxide 平台绑定。但 CHANGELOG v0.3.0 提到 "Removed unused Tailwind CSS import"，组件全部使用 scoped CSS + Catppuccin CSS 变量。

**涉及文件**：
- `apps/desktop/package.json`
- `apps/desktop/vite.config.ts`（需要确认）

**操作**：
1. `grep -r "tailwind\|@apply\|@tailwind\|@layer" apps/desktop/src/` 确认是否还有 tailwind 类使用
2. 如果没有使用，从 package.json 移除 tailwindcss、@tailwindcss/vite 和 4 个 optional 平台依赖
3. 从 vite.config.ts 移除 tailwind 插件引用
4. 如果确认未使用但在构建中有隐式作用，保留并添加注释说明

**验收标准**：
- `npm install` 不再安装未使用的 tailwind 依赖
- `npm run build` 和 `npm run tauri build` 成功

---

## 14.6 🟡 P2 `toml` crate 在 hook-adapter 中未使用

**问题**：[hook-adapter/Cargo.toml](../../adapters/hook-adapter/Cargo.toml#L21) 声明了 `toml = "0.8"` 但 `installer.rs` 中自己手写了 TOML section parser，没有使用 `toml` crate。

**涉及文件**：
- `adapters/hook-adapter/Cargo.toml`

**操作**：
- 移除 `toml = "0.8"` 依赖（或改为使用 `toml` crate 替换手写 parser，如果手写逻辑复杂）

**验收标准**：
- `cargo build` 成功
- 所有 hook-adapter 测试通过

---

## 14.7 🟡 P2 事件服务器 shutdown signal 未被使用

**问题**：[lib.rs:144-146](../../apps/desktop/src-tauri/src/lib.rs#L144-L146) 接收了 `start_shared` 返回的 `shutdown` Arc\<AtomicBool\> 但用 `let _ = ...` 丢弃。应用退出时 HTTP 服务器线程无法被通知优雅关闭。

**涉及文件**：
- `apps/desktop/src-tauri/src/lib.rs`

**操作**：
1. 将 shutdown signal 保存到 `AppState` 或应用级结构中
2. 在 Tauri `on_exit` 或应用 Drop 时调用 `shutdown.store(true, Ordering::Relaxed)`

**验收标准**：
- 关闭 AgentPulse 窗口时 HTTP 服务器线程能在下一次循环迭代中退出
- 不会出现端口未释放的僵尸线程

---

## 关联

- [[13-health-check-phase1-critical-fixes]] — 第一阶段
- [[13-health-check-phase3-polish]] — 第三阶段
- [[08-infra-improvements]] — tiny_http、tauri-action、缺少代码签名
- [[03-code-quality]] — 3.4 冗余测试、3.5 IPC 类型安全
- [[09-frontend-type-safety]] — IPC 类型安全和 DB 测试去重
