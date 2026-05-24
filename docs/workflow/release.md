# Release Workflow

## 概述

推送 `v*` 格式的 tag 到 GitHub 后，GitHub Actions 自动触发跨平台构建，生成安装包并发布为 GitHub Release。

## 触发条件

- 推送 tag 匹配 `v*`（如 `v0.1.0`, `v1.0.0`）

## 构建产物

| 平台 | 产物 |
|------|------|
| Windows (x64) | `.msi` + `.exe` (NSIS installer) |
| macOS (x64 / arm64) | `.dmg` |
| Linux (x64) | `.deb` + `.AppImage` |

## 发布步骤

```powershell
# 1. 确保代码已提交
git status

# 2. 提交所有改动
git add -A
git commit -m "chore: prepare release v0.1.0"

# 3. 推送代码
git push origin master

# 4. 创建 tag
git tag v0.1.0

# 5. 推送 tag（触发构建）
git push origin v0.1.0
```

## Workflow 流程

1. **Checkout** — 拉取代码
2. **Setup Node.js** — 安装 Node.js 18
3. **Setup Rust** — 安装 Rust stable toolchain，配置跨平台 target
4. **Install Linux dependencies**（仅 Linux）— `webkit2gtk-4.1`, `ayatana-appindicator` 等
5. **npm ci** — 安装前端依赖
6. **tauri-action** — 构建安装包 + 创建 GitHub Release draft

## 发布后操作

1. 访问 [GitHub Releases](https://github.com/Kal-zed/AgentPulse/releases)
2. 检查构建产物是否完整
3. 编辑 release notes
4. 点击 **Publish release**

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
