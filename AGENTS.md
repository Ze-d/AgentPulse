# AGENTS.md

## Project: AgentPulse

## Instructions

- Write tests first (TDD), verify they fail, then implement
- Run tests before committing
- Follow existing code patterns in the project
- Keep changes minimal and focused
- 使用中文回答用户，代码注释使用英文

## Git Workflow

**核心原则：feature branch → PR → squash merge → master**

### 分支策略

- `master` — 稳定分支，只接受 squash merge，不直接提交
- `feat/<name>` — 新功能分支，从 master 拉出
- `fix/<name>` — 修复分支，从 master 拉出
- `chore/<name>` — 杂项（依赖、配置、CI）

### 合并前 CI 检查

**每次合并到 master 前，必须先通过本地 CI 检查：**

```powershell
# 运行完整 CI 检查（含安全审计）
.\scripts\ci-check.ps1 -Full

# 快速检查（仅编译 + 测试，跳过审计）
.\scripts\ci-check.ps1 -Quick
```

**检查项：**
| 步骤 | 命令 | 说明 |
|------|------|------|
| TypeScript 类型检查 | `vue-tsc --noEmit` | 确保类型正确 |
| 前端测试 | `npm test` (Vitest) | 运行前端单元测试 |
| Python 测试 | `pytest tests/` | 运行 Python 单元测试 |
| Rust 格式检查 | `cargo fmt --check` | 确保代码格式一致 |
| Rust Lint | `cargo clippy -- -D warnings` | 零警告 |
| Rust 测试 | `cargo test` | 运行 Rust 单元测试 |
| 依赖审计 | `cargo audit` + `npm audit` | 仅 `-Full` 模式 |

**CI 通过了才能：**
1. `git push origin <branch>`
2. 创建 PR
3. Squash merge 到 master

**GitHub 侧保护：** master 分支已配置要求 CI 通过后才能合并。
首次设置时运行 `.\scripts\setup-branch-protection.ps1` 或手动在
https://github.com/Ze-d/AgentPulse/settings/branches 配置。

### 日常工作流

```powershell
# 1. 从 master 拉分支
git checkout master
git pull origin master
git checkout -b feat/my-feature

# 2. 在分支上随意提交（wip、fix、docs 都可以）
git commit -m "feat: core logic"
git commit -m "docs: update readme"
# ...

# 3. 完成后，运行本地 CI 检查（必须全部通过）
.\scripts\ci-check.ps1

# 4. 推送分支，创建 PR
git push origin feat/my-feature
gh pr create --title "feat: <简短描述>"

# 5. 等待 GitHub CI 通过后，squash merge 合入 master
gh pr merge --squash --delete-branch
```

### Commit 规范

- 使用 [Conventional Commits](https://www.conventionalcommits.org/)：`type: description`
- 类型：`feat` / `fix` / `docs` / `chore` / `test` / `style` / `refactor`
- **分支上的 wip commit 无所谓**，合入 master 时会被 squash 成一条干净提交

### 为什么

- `master` 历史线性、一条一个功能，方便 `git bisect` 和 review
- 分支上可以随意提交，降低心理负担
- squash merge 后自动删除远程分支，保持仓库整洁

## Test Commands

```powershell
# Python unit tests
python -m pytest tests/unit/ -v

# Python E2E test (requires AgentPulse running)
python tests/integration/test_e2e.py

# Rust tests
cd apps/desktop/src-tauri && cargo test

# TypeScript type check
cd apps/desktop && npx vue-tsc --noEmit
```

## Key Files

| File | Purpose |
|------|---------|
| `adapters/claude-code/install_hooks.py` | Hook install/uninstall/status CLI |
| `adapters/claude-code/monitor_hook.py` | Hook event stdin→HTTP adapter |
| `apps/desktop/src-tauri/src/lib.rs` | Shared types + app entry point |
| `apps/desktop/src-tauri/src/db.rs` | SQLite database |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP event server :17878 |
| `apps/desktop/src-tauri/src/state_machine.rs` | State transitions |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri IPC commands |
| `apps/desktop/src/stores/sessionStore.ts` | Frontend state + polling |
| `apps/desktop/src/components/FloatingPanel.vue` | Main floating panel |
