# TODO: 功能增强

> 提升用户体验和实用性的新功能，按优先级排列

---

## 2.1 数据库持久化存储 🟡

**问题**: 当前使用 `Database::new_in_memory()`，应用重启后所有数据丢失。作为监控工具，用户期望看到历史 session。

**文件**: [apps/desktop/src-tauri/src/lib.rs](../../apps/desktop/src-tauri/src/lib.rs)

**建议**:
- 改为文件 SQLite（`Connection::open(app_data_dir.join("agentpulse.db"))`）
- 添加 schema migration 机制（或在启动时执行 `CREATE TABLE IF NOT EXISTS`）
- 考虑定期清理过期 session（如保留最近 7 天）

---

## 2.2 needs_attention 可视化 🟡

**问题**: `attentionSessions` getter 已实现、`needsAttention` 字段已有数据流通，但 UI 无任何视觉呈现。

**文件**:
- [apps/desktop/src/stores/sessionStore.ts](../../apps/desktop/src/stores/sessionStore.ts)
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)

**建议**: 在 SessionCard 上为 `needsAttention: true` 的 session 添加脉冲边框动画或醒目的指示点。等待 permission/input 的 session 应更容易被注意到。

---

## 2.3 Session 卡片展开/折叠动画 🟡

**问题**: 卡片展开/折叠瞬间切换，无过渡动画。视觉体验突兀。

**建议**: 使用 Vue `<Transition>` / `<TransitionGroup>` 包裹 session list 和 ExpandedDetail，添加 `transition: all 0.2s ease`。

---

## 2.4 加载状态指示器 🟡

**问题**: 应用启动时、polling 首响应之前，用户看到 "waiting for hooks..." 空状态，无法区分「加载中」和「无数据」。

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)

**建议**:
- Store 添加 `isLoading: boolean` 状态
- 初始为空时显示闪烁的 `_` 光标动画，模仿终端
- 也可以显示 skeleton

---

## 2.5 手动刷新按钮 🟢

**问题**: 仅依赖 2 秒轮询，用户无法主动刷新。若怀疑数据过期，需等待最多 2 秒。

**建议**: header 区添加微小的刷新图标（`↻`），点击时立即触发 `fetchSessions()`。

---

## 2.6 错误 banner 可关闭 🟢

**问题**: 错误 banner 无关闭按钮，只能等待下次 poll 成功后自动清除。

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)

**建议**: 添加 `×` 关闭按钮，点击后 `store.error = null`。

---

## 2.7 用户友好的空状态文案 🟢

**问题**: 空状态文案 "$ waiting for hooks..." 面向开发者。新用户不理解什么是 hooks。

**建议**: 改为 `$ agentpulse is listening...` 或类似更通用的描述。

---

## 2.8 关闭按钮语义优化 🟢

**问题**: 关闭按钮显示 "x" 且 `title="Close"`，但实际行为是隐藏到托盘而非退出。标准 "x" 暗示退出。

**建议**: 改为 `title="Minimize to tray"`，或使用 `—` / `_` 图标表示最小化。

---

## 2.9 项目名 tooltip 🟢

**问题**: `SessionCard.vue` 的 `.project` 使用了 `text-overflow: ellipsis`，但没有 `title` 属性显示完整名称。而 ExpandedDetail 的 `cwd` 正确设置了 `title`。

**文件**: [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)

**建议**: 给 `.project` span 添加 `:title="session.projectName"`。

---

## 2.10 "Open dir" / "Transcript" 失败反馈 🟢

**问题**: `handleOpenDir` 和 `handleOpenTranscript` 失败时仅 `console.error`，用户无感知。

**建议**: 失败时设置 `store.error` 显示错误 banner。

---

## 2.11 多源 Agent 支持 🟢

**问题**: `AgentSource` 枚举仅定义了 `ClaudeCode`。架构上预留了扩展空间但未实现。

**文件**: [apps/desktop/src-tauri/src/lib.rs](../../apps/desktop/src-tauri/src/lib.rs)

**建议**: 为下一个 Agent 源（如 Codex CLI、Gemini CLI、Copilot CLI）添加适配器骨架。
