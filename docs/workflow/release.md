# Release Workflow

## 概述

完整的发布流程分为两个阶段：

1. **合并前 CI 检查** — 确保代码质量，防止合并后 CI 持续报错
2. **构建与发布** — 推送 `v*` tag 触发 GitHub Actions 跨平台构建

---

## 第一阶段：合并前 CI 检查

### 本地 CI 检查

**每次合并到 master 前，必须先通过本地 CI 检查：**

```powershell
# 完整检查（含安全审计）
.\scripts\ci-check.ps1 -Full

# 快速检查（仅编译 + 测试）
.\scripts\ci-check.ps1 -Quick
```

检查项：

| 步骤 | 命令 | 说明 |
|------|------|------|
| TypeScript 类型检查 | `vue-tsc --noEmit` | 确保类型正确 |
| 前端测试 | `npm test` (Vitest) | 运行前端单元测试 |
| Python 测试 | `pytest tests/` | 运行 Python 单元测试 |
| Rust 格式检查 | `cargo fmt --check` | 确保代码格式一致 |
| Rust Lint | `cargo clippy -- -D warnings` | 零警告 |
| Rust 测试 | `cargo test` | 运行 Rust 单元测试 |
| 依赖审计 | `cargo audit` + `npm audit` | 仅 `-Full` 模式 |

### GitHub CI 检查

推送分支后，GitHub Actions 自动在 **3 个平台** 上运行完整 CI：

- **windows-latest**
- **ubuntu-latest**
- **macos-latest**

CI 配置文件：[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)

触发条件：
- `pull_request` 到 `master`
- `push` 到 `master`
- `merge_group`（GitHub Merge Queue）
- `workflow_dispatch`（手动触发）

### 分支保护

master 分支已配置要求 CI 通过后才能合并。

首次设置：
```powershell
.\scripts\setup-branch-protection.ps1
```

或手动在 https://github.com/Ze-d/AgentPulse/settings/branches 配置，要求以下 status check 全部通过：

- `Check (windows-latest)`
- `Check (ubuntu-latest)`
- `Check (macos-latest)`

---

## 第二阶段：构建与发布

### 触发条件

推送 `v*` 格式的 tag 到 GitHub（如 `v0.1.0`, `v1.0.0`）触发 `.github/workflows/release.yml`。

### 构建产物

| 平台 | 产物 |
|------|------|
| Windows (x64) | `.msi` + `.exe` (NSIS installer) |
| macOS (x64 / arm64) | `.dmg` |
| Linux (x64) | `.deb` + `.AppImage` |

### 发布步骤

```powershell
# 1. 确保在 feature 分支上，CI 已通过
.\scripts\ci-check.ps1

# 2. 推送分支并创建 PR
git push origin feat/my-feature
gh pr create --title "feat: xxx"

# 3. 等待 GitHub CI 全部通过后，squash merge 合入 master
gh pr merge --squash --delete-branch

# 4. 切回 master 并同步
git checkout master
git pull origin master

# 5. 更新版本号并提交（如需手动更新）
# 编辑 Cargo.toml / package.json 版本号
git add -A
git commit -m "chore: bump version to v0.1.0"
git push origin master

# 6. 创建 tag
git tag v0.1.0

# 7. 推送 tag（触发构建）
git push origin v0.1.0
```

### Release Workflow 流程

1. **test job**（门禁）— TypeScript 类型检查、前端测试、Python 测试、Rust fmt/clippy/test
2. **build job**（依赖 test 通过）— 4 个平台/目标并行构建安装包 → 创建 GitHub Release draft

### 发布后操作

1. 访问 [GitHub Releases](https://github.com/Ze-d/AgentPulse/releases)
2. 检查构建产物是否完整
3. 编辑 release notes
4. 点击 **Publish release**

---

## 签名（可选）

若要启用 Windows/macOS 代码签名，在 GitHub Secrets 中配置：

| Secret | 说明 |
|--------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater 签名私钥 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码 |

不配置签名不影响安装包生成（macOS 用户需手动绕过 Gatekeeper）。

## 手动构建（本地）

```powershell
cd apps/desktop
npm run tauri build
```

产物输出到 `apps/desktop/src-tauri/target/release/bundle/`。
