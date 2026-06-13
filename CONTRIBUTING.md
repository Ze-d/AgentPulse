# Contributing to AgentPulse

感谢你对 AgentPulse 的关注！本文档将帮助你快速上手开发。

## 开发环境搭建

详见 [本地开发指南](docs/local-development-guide.md)。

### 环境要求

| 工具 | 最低版本 | 用途 |
|------|---------|------|
| Node.js | >= 18 | 前端构建 |
| Rust | >= 1.70 (MSVC toolchain) | Tauri 后端 + Hook 适配器 |
| Git | 任意 | 版本控制 |

### 快速开始

```powershell
git clone https://github.com/Ze-d/AgentPulse.git
cd AgentPulse/apps/desktop
npm install
cd src-tauri && cargo check && cd ..
npm run tauri dev
```

## 分支策略与 Commit 规范

详见 [AGENTS.md](AGENTS.md)。

- `master` — 稳定分支，只接受 squash merge
- `feat/<name>` — 新功能分支
- `fix/<name>` — 修复分支
- `chore/<name>` — 杂项（依赖、配置、CI）

Commit 格式：`type: description` (Conventional Commits)

| Type | 用途 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档更新 |
| `chore` | 依赖/配置/CI |
| `test` | 测试 |
| `refactor` | 重构 |

## 运行测试

```powershell
# Rust 测试
cd apps/desktop/src-tauri
cargo test

# Hook 适配器测试
cd adapters/hook-adapter
cargo test

# 前端测试
cd apps/desktop
npm test

# TypeScript 类型检查
cd apps/desktop
npx vue-tsc --noEmit
```

## CI 检查

合并到 master 前，请确保以下检查全部通过：

```powershell
# 完整 CI 检查
.\scripts\ci-check.ps1 -Full

# 快速检查（仅编译 + 测试）
.\scripts\ci-check.ps1 -Quick
```

## PR 提交流程

1. 从 `master` 拉出功能分支
2. 在分支上开发和提交
3. 运行本地 CI 检查
4. 推送分支，创建 PR
5. 等待 GitHub CI 通过
6. Squash merge 合入 master

### PR 标题格式

```
feat: 简短描述
fix: 简短描述
chore: 简短描述
```

🤖 Generated with [Claude Code](https://claude.com/claude-code)
