# TODO: 测试与 CI 增强

> CI 流水线完整性、测试覆盖率、发布流程

---

## 5.1 CI 中运行 Python 测试 🟡

**问题**: GitHub Actions CI 完全跳过了 Python 测试（30 个单元测试 + E2E test）。`monitor_hook.py` 或 `install_hooks.py` 退化不会被捕获。

**文件**: [.github/workflows/ci.yml](../../.github/workflows/ci.yml)

**建议**: 在 CI 中添加 `python tests/unit/test_install_hooks.py && python tests/unit/test_monitor_hook.py`。

---

## 5.2 前端单元测试缺失 🟡

**问题**: 项目中无前端测试框架（Vitest / vue-test-utils / Playwright）。Vue 组件没有任何单元测试。

**建议**:
- 安装 Vitest + @vue/test-utils
- 为 `sessionStore.ts` 编写单元测试（actions / getters）
- 为 `formatDuration`、`STATUS_LABELS` 等纯函数编写单元测试

---

## 5.3 CI 添加 lint 检查 🟡

**问题**: 无 ESLint、Rustfmt、Clippy 检查。代码风格完全依赖开发者自律。

**建议**:
- 添加 `cargo clippy -- -D warnings`
- 添加 `cargo fmt --check`
- 考虑添加 ESLint（或依赖 `vue-tsc` 的类型检查作为替代）

---

## 5.4 CI 跨平台 🟡

**问题**: CI 仅在 `windows-latest` 运行。Linux/macOS 特定问题在 push 后才发现（或通过 release workflow 发现）。

**文件**: [.github/workflows/ci.yml](../../.github/workflows/ci.yml)

**建议**: 在 CI matrix 中添加 `ubuntu-latest` 和 `macos-latest`（或仅在适当目标上运行 Rust 检查）。

---

## 5.5 依赖安全审计 🟡

**问题**: CI 中无 `cargo audit` 或 `npm audit`。已知漏洞不会被自动发现。

**建议**: 添加 `cargo audit` 和 `npm audit --audit-level=high` 步骤。

---

## 5.6 `tiny_http` 长期未维护 🟡

**问题**: `tiny_http` 0.12 最后更新于 2022 年。可能有未修补的漏洞和 bug。

**文件**: [apps/desktop/src-tauri/Cargo.toml](../../apps/desktop/src-tauri/Cargo.toml)

**建议**: 评估替换为 `actix-web`、`axum` 或 `warp`（增加依赖量但更可靠）。

---

## 5.7 Release 前无 CI 门控 🟡

**问题**: Release workflow 不运行任何测试/检查，直接在 tag push 后构建。破损代码可以被发布。

**文件**: [.github/workflows/release.yml](../../.github/workflows/release.yml)

**建议**: 在 release build 前添加 `cargo test` 和 `cargo clippy` 步骤。

---

## 5.8 CHANGELOG.md 缺失 🟢

**问题**: Release workflow 引用了 `CHANGELOG.md`，但文件不存在。

**建议**: 创建 CHANGELOG.md 并遵循 [Keep a Changelog](https://keepachangelog.com/) 格式，或使用自动生成工具。

---

## 5.9 CI `npm install` 未使用 `npm ci` 🟢

**问题**: CI 使用 `npm install` 而非 `npm ci`。`package-lock.json` 未被验证为最新，可能引入漂移。

**文件**: [.github/workflows/ci.yml](../../.github/workflows/ci.yml)

**建议**: 改为 `npm ci --include=optional`。

---

## 5.10 Release 使用过时的 tauri-action 🟢

**问题**: `release.yml` 使用 `tauri-apps/tauri-action@v0`，当前版本是 `v2`。

**建议**: 迁移到 `tauri-apps/tauri-action@v2`。

---

## 5.11 无代码签名 🟢

**问题**: Windows/macOS 发布二进制文件未签名。用户安装时会看到安全警告。

**建议**: 配置 Windows Authenticode 签名和 macOS 公证（notarization）。
