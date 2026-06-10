# 09 — 前端类型安全与测试整理

**状态：** 待规划  
**优先级：** 中  
**创建日期：** 2026-06-10

---

## 9.1 IPC 调用无类型安全 🔴

**问题**: `ipc.ts` 中使用 `invoke<T>("command_name")` 字符串匹配。命令名拼写错误（如 `"get_sessions"` 写成 `"get_session"`）仅在运行时才发现，无编译期检查。

**涉及文件**:
- `apps/desktop/src/utils/ipc.ts`

**当前状态**（来自 03-code-quality 3.5 任务 — 未完成）:

```typescript
// 当前方式
const sessions = await invoke<AgentSession[]>("get_sessions");
await invoke("hide_main_window");
```

**推荐方案**:

### 方案 A：手写 typed wrapper 层（推荐）

在 `ipc.ts` 中封装一层类型安全的函数：

```typescript
// apps/desktop/src/utils/ipc.ts

export async function getSessions(): Promise<AgentSession[]> {
  return invoke<AgentSession[]>("get_sessions");
}

export async function hideMainWindow(): Promise<void> {
  return invoke("hide_main_window");
}

export async function deleteSession(sessionId: string): Promise<void> {
  return invoke("delete_session", { sessionId });
}

// ... 所有 IPC 命令集中管理
```

- **优点**: 简单、无额外依赖、编译期检查函数名
- **缺点**: 仍无法检查 Rust 侧命令签名是否匹配（需手动维护）

### 方案 B：tauri-specta（全自动）

使用 [tauri-specta](https://github.com/specta-rs/tauri-specta) 从 Rust 命令自动生成 TypeScript 绑定：

```rust
// Rust 侧
#[tauri::command]
#[specta::specta]
fn get_sessions(state: AppState) -> Vec<AgentSession> { ... }
```

```typescript
// 自动生成的 TypeScript
import { commands } from "./bindings";
const sessions = await commands.getSessions(); // 完全类型安全
```

- **优点**: Rust 签名变更自动同步到前端、零维护成本
- **缺点**: 增加构建步骤和依赖、项目侵入性较高

### 方案 C：tauri-typed（中间方案）

使用社区 `tauri-typed` macro 生成类型。

**推荐**: 方案 A（手写 wrapper），改动最小、风险最低，且与现有代码风格一致（CHANGELOG 提到 "IPC calls wrapped in typed functions" 已部分完成）。

---

## 9.2 DB 模块内测试与集成测试去重 🟡

**问题**（来自 03-code-quality 3.4 任务 — 未完成）:

`db.rs` 中有 14 个 `#[cfg(test)]` 模块内单元测试，原计划迁移到独立的 `tests/db_test.rs` 集成测试，但该文件不存在。

**涉及文件**:
- `apps/desktop/src-tauri/src/db.rs` — 14 个模块内测试
- 需要创建: `apps/desktop/src-tauri/tests/db_test.rs`

**现状分析**:
- 模块内测试覆盖充分（14 个 case），没必要删除重写
- 集成测试更适合验证跨模块行为（DB + event_server + state_machine）

**建议**: 保留 DB 模块内测试（它们工作正常），改为创建 **集成测试** 覆盖跨模块场景：
- `tests/integration_test.rs`: 完整的 session 生命周期（create → upsert → events → cleanup）
- `tests/integration_test.rs`: Hook install + event 接收 + DB 写入 端到端

这样不浪费现有的模块内测试覆盖，同时补齐集成层测试空缺。

---

## 9.3 前端依赖版本不一致 🟢

**问题**: 根目录 `package.json` 和 `apps/desktop/package.json` 中存在版本冲突：

| 包 | 根 package.json | apps/desktop/package.json |
|----|----------------|--------------------------|
| `vitest` | `^4.1.7` | `^2.1.8` |
| `@vue/test-utils` | `^2.4.10` | `^2.4.6` |
| `happy-dom` | `^20.9.0` | `^16.0.0` |

根 `package.json` 只有 `devDependencies`，没有 `scripts`，看起来是残留或误放。

**涉及文件**:
- `package.json` (根目录)

**建议**: 
1. 清理根 `package.json` 的 devDependencies（它们未被任何 `npm` 命令使用）
2. 如果根 package.json 有特定用途，统一到与 `apps/desktop` 一致的版本

---

## 关联

- [[03-code-quality]] — 3.4（冗余测试）、3.5（IPC 类型安全）
- [[08-infra-improvements]] — CI/Release 配置修复
