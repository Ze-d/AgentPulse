# Fixlog — 2026-05-29

Feature enhancements from [02-feature-enhancements.md](../todos/02-feature-enhancements.md) and bugfixes discovered during implementation.

## Fixed Issues

### 2.10 "Open dir" 打开网页而非文件夹 🔴

- **文件**: [FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue#L25-L32)
- **根因**: `handleOpenDir` 使用 `openUrl(convertFileSrc(cwd))` 将目录路径转为 `file://` URL 然后在浏览器中打开，而非在文件管理器中打开
- **修复**:
  - 新建 [openActions.ts](../../apps/desktop/src/utils/openActions.ts)，用 `openPath()` 替代 `openUrl(convertFileSrc(...))`
  - `handleOpenDir` → `openDirectory(cwd)` 用系统文件管理器打开
  - `handleOpenTranscript` → `openTranscript(path)` 用默认编辑器打开
  - 失败时设置 `store.error` 显示 error banner（同时修复 2.10 的失败反馈问题）
- **新增测试**: 4 个 ([openActions.test.ts](../../apps/desktop/src/utils/__tests__/openActions.test.ts))
  - `openDirectory` 调用 `openPath` 而非 `openUrl`
  - `openDirectory` 失败时抛出可被捕获的错误
  - `openTranscript` 调用 `openPath` 而非 `openUrl`
  - `openTranscript` 失败时抛出可被捕获的错误

### 2.6 错误 banner 无法关闭 🟢

- **文件**: [sessionStore.ts](../../apps/desktop/src/stores/sessionStore.ts)、[FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)
- **根因**: error banner 无关闭按钮，只能等待下次 poll 成功后自动清除
- **修复**: Store 新增 `clearError()` action，error banner 添加 `×` 关闭按钮
- **新增测试**: 2 个 — `clearError` 设置 error 为 null / 对 null 无操作

### 2.5 无手动刷新按钮 🟢

- **文件**: [FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)
- **根因**: 仅依赖 2 秒轮询，用户无法主动刷新
- **修复**: Header 添加 `↻` 按钮，点击触发 `store.fetchSessions()`

### 2.4 无加载状态指示 🟡

- **文件**: [sessionStore.ts](../../apps/desktop/src/stores/sessionStore.ts)、[FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)
- **根因**: 启动时无法区分「加载中」和「无数据」
- **修复**:
  - Store 新增 `isLoading: boolean`，初始 `true`，`fetchSessions` 的 `finally` 设 `false`
  - 加载中显示闪烁 `_` 光标动画（`@keyframes blink`），加载完成显示 "agentpulse is listening..."
- **新增测试**: 3 个 — 初始 true / fetch 后 false / fetch 失败后仍 false

### 2.11 多源 Agent 支持 🟢

- **文件**: [lib.rs](../../apps/desktop/src-tauri/src/lib.rs)、[agent.ts](../../apps/desktop/src/types/agent.ts)、[sourceDisplay.ts](../../apps/desktop/src/utils/sourceDisplay.ts)
- **根因**: `AgentSource` 仅定义了 `ClaudeCode`，架构预留了扩展空间但未实现
- **修复**:
  - Rust 枚举新增 `Codex`、`Gemini`、`Copilot` 变体（含 serde rename）
  - TypeScript 类型扩展为 `"claude-code" | "codex" | "gemini" | "copilot"`
  - 新建 `sourceAbbr()` 工具函数映射缩写（cc/cx/gm/cp），SessionCard 使用该函数替代 hardcode
- **新增测试**: 5 个 TypeScript + 1 个 Rust (`test_agent_source_new_variants_roundtrip`)

### 2.2 needs_attention 可视化 🟡

- **文件**: [SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)
- **根因**: `attentionSessions` getter 和 `needsAttention` 字段已就绪，但 UI 无视觉呈现
- **修复**: SessionCard 添加 `:class="{ attention: session.needsAttention }"` → `@keyframes attention-pulse` 琥珀色脉冲阴影
- **新增测试**: 2 个 — `attentionSessions` getter 过滤逻辑

### 2.3 卡片展开/折叠动画 🟡

- **文件**: [FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)
- **根因**: 卡片展开/折叠瞬间切换，无过渡
- **修复**: ExpandedDetail 用 `<Transition name="slide">` 包裹，`v-if`/`v-else` 改为独立 `v-if` → `opacity + translateY` 0.2s ease

### 2.7–2.9 文案和交互优化 🟢

- **2.7**: 空状态文案 `"waiting for hooks..."` → `"agentpulse is listening..."`
- **2.8**: 关闭按钮 `title="Close"` → `title="Minimize to tray"`，图标 `x` → `_`
- **2.9**: SessionCard `.project` span 添加 `:title="session.projectName"` tooltip

### Process Checker 误删已完成 session 🔴

- **文件**: [process_checker.rs](../../apps/desktop/src-tauri/src/process_checker.rs)
- **根因**: process checker 只要 PID 不存在就删除 session。cc 进程退出后 PID 消失 → completed session 被立即删除 → 面板看不到 "done" 状态
- **修复**: 新增 `is_active_status()` 函数，`Completed`/`Failed`/`Unknown` 视为终端状态跳过删除，由 cleanup 统一处理
- **新增测试**: `test_is_active_status_excludes_terminal` — 8 个状态的断言

### monitor_hook.py 重复死代码 🟡

- **文件**: [monitor_hook.py](../../adapters/claude-code/monitor_hook.py)
- **根因**: `_snapshot_processes()` 函数 `return` 之后有 `read_stdin()` 的残留副本（复制粘贴错误），永远不会执行
- **修复**: 删除死代码

### State Machine 终态无法恢复 🔴

- **文件**: [state_machine.rs](../../apps/desktop/src-tauri/src/state_machine.rs#L23-L34)
- **根因**: `Completed`/`Failed` 状态下收到新事件（如 `PreToolUse`）时无匹配规则，走 `_ => current` 保持终态不变。用户继续对话后 session 永远卡在 "done"
- **修复**:
  - `PreToolUse` 新增 `Completed`/`Failed` → `ToolRunning` 转换
  - 新增通用恢复规则：`Completed`/`Failed` 收到任何其他事件 → `Running`（放在 `Notification` 之后确保 Notification 优先匹配 `WaitingInput`）
- **新增测试**: 4 个 — Completed/Failed 在 PreToolUse、Notification、SessionStart 下的恢复

## 验证结果

```
TypeScript: 18 passed | Rust: 29 passed | vue-tsc: 0 errors
```

| Suite | Tests | Result |
|-------|-------|--------|
| openActions.test.ts | 4 | all pass |
| sourceDisplay.test.ts | 5 | all pass |
| sessionStore.test.ts | 9 | all pass |
| lib unit tests (Rust) | 25 | all pass |
| db_test (Rust integ) | 3 | all pass |
| event_server_test | 5 | all pass |
| state_machine_test | 11 | all pass |
| types_test | 3 | all pass |

## TDD 新增测试汇总

| 测试文件 | 测试数 | 覆盖 |
|---------|--------|------|
| `openActions.test.ts` | 4 | openPath vs openUrl, 错误抛出 |
| `sourceDisplay.test.ts` | 5 | 多源缩写映射 + 未知回退 |
| `sessionStore.test.ts` | 9 | clearError, attentionSessions, isLoading, fetchSessions |
| `db.rs (Rust)` | 2 | 新变体 roundtrip, file persistence |
| `process_checker.rs (Rust)` | 1 | is_active_status 终端状态排除 |
| `state_machine_test.rs (Rust)` | 4 | Completed/Failed 终态恢复 |
