# ~~TODO: 测试与 CI 增强~~ (8/11 完成)

> 状态：**8/11 已完成** — 5.6, 5.10, 5.11 未完成（验证日期: 2026-06-07）

---

## 5.1 CI 中运行 Python 测试 ✅

**已完成**: `ci.yml` L58-61 添加了 `python -m pytest tests/` 步骤。

---

## 5.2 前端单元测试缺失 ✅

**已完成**: `apps/desktop/src/utils/__tests__/` 目录下有前端测试文件（`sourceDisplay.test.ts`, `openActions.test.ts`）。`ci.yml` L54-56 运行 `npm test`。

---

## 5.3 CI 添加 lint 检查 ✅

**已完成**: `ci.yml` L63-69 添加 `cargo fmt --check` 和 `cargo clippy -- -D warnings`。

---

## 5.4 CI 跨平台 ✅

**已完成**: `ci.yml` L13-16 的 matrix 包含 `windows-latest`, `ubuntu-latest`, `macos-latest`。

---

## 5.5 依赖安全审计 ✅

**已完成**: `ci.yml` L75-83 添加 `cargo audit` 和 `npm audit --audit-level=high`。

---

## 5.6 `tiny_http` 长期未维护 🔴 未完成

**问题**: 仍在使用 `tiny_http`。`Cargo.toml` 未替换。

**建议**: 评估替换为 `actix-web`、`axum` 或 `warp`。

---

## 5.7 Release 前无 CI 门控 ✅

**已完成**: `release.yml` L13-52 添加 `test` job（含 TypeScript 检查、前端测试、cargo fmt/clippy/test），L54 `build` job 通过 `needs: test` 依赖测试通过后才构建。

---

## 5.8 CHANGELOG.md 缺失 ✅

**已完成**: 根目录 `CHANGELOG.md` 已创建。

---

## 5.9 CI `npm install` 未使用 `npm ci` ✅

**已完成**: `ci.yml` L48 使用 `npm ci --include=optional`。

---

## 5.10 Release 使用过时的 tauri-action 🔴 未完成

**问题**: `release.yml` L124 仍使用 `tauri-apps/tauri-action@v0`，当前版本是 `v2`。

**建议**: 迁移到 `tauri-apps/tauri-action@v2`。

---

## 5.11 无代码签名 🔴 未完成

**问题**: Windows/macOS 发布二进制文件未签名。

**建议**: 配置 Windows Authenticode 签名和 macOS 公证。
