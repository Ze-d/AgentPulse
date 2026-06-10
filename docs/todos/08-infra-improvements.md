# 08 — 基础设施改进

**状态：** 待规划  
**优先级：** 高  
**创建日期：** 2026-06-10

---

## 8.1 替换 `tiny_http` 为现代 HTTP 框架 🔴

**问题**: `Cargo.toml` 中仍使用 `tiny_http = "0.12"`，该 crate 已长期未维护（最后更新距今超过 2 年），存在潜在安全风险和生态脱节。

**涉及文件**:
- `apps/desktop/src-tauri/Cargo.toml` — 依赖替换
- `apps/desktop/src-tauri/src/event_server.rs` — HTTP server 实现重写

**候选方案**:

| 框架 | 优点 | 缺点 |
|------|------|------|
| `axum` | 生态最活跃、async/await 原生、tower 中间件 | 需要 tokio runtime（Tauri 已内置） |
| `actix-web` | 成熟稳定、性能优秀 | 自带 actor runtime，与 Tauri tokio 并存增加复杂度 |
| `warp` | 轻量、filter 组合灵活 | 社区活跃度下降 |

**推荐**: `axum` — Tauri 2 本身基于 tokio，axum 可复用同一 runtime，且生态最好。

**预估改动**:
- `event_server.rs` 从同步 `tiny_http` 改为 async `axum` router
- 移除 `std::thread::spawn` 包装，改用 `tokio::spawn`
- `start()` 函数签名可能需要变为 async

---

## 8.2 Release Workflow 升级 `tauri-action` 🔴

**问题**: `release.yml` 第 124 行仍使用 `tauri-apps/tauri-action@v0`，当前最新稳定版是 `v2`。

**涉及文件**:
- `.github/workflows/release.yml`

**改动**:
```diff
- uses: tauri-apps/tauri-action@v0
+ uses: tauri-apps/tauri-action@v2
```

> ⚠️ `v2` 的配置项可能有 breaking changes，需对照 [tauri-action v2 文档](https://github.com/tauri-apps/tauri-action) 确认参数兼容性。

---

## 8.3 缺少代码签名 🟡

**问题**: Windows/macOS 发布二进制文件未签名。Windows 上 SmartScreen 会警告用户，macOS 上 Gatekeeper 会阻止运行。

**涉及文件**:
- `.github/workflows/release.yml`

**建议**:

**Windows Authenticode**:
- 需要 OV/EV Code Signing Certificate（约 $200-400/年）
- 或使用 Azure Key Vault + `azure-sign-tool` 托管签名
- 在 release workflow 中添加 `signavitae` 或 `signcode` step

**macOS 公证**:
- 需要 Apple Developer Program 会员（$99/年）
- 在 release workflow 中传递 `APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_ID` 等 secrets 给 tauri-action
- tauri-action v2 原生支持 macOS 签名和公证

**注意**: 代码签名需要费用和证书申请，属于发布前最后一步，可以暂时跳过。

---

## 8.4 Release Test Job 仅测 Windows 🟡

**问题**: `release.yml` 的 `test` job 硬编码 `runs-on: windows-latest`。Linux 和 macOS 构建前不跑测试。

**涉及文件**:
- `.github/workflows/release.yml` (L13)

**建议**: 将 `test` job 改为 matrix strategy（同 CI）：
```yaml
test:
  strategy:
    fail-fast: false
    matrix:
      platform: [windows-latest, ubuntu-latest, macos-latest]
  runs-on: ${{ matrix.platform }}
```

或至少加上 `ubuntu-latest`（Linux 用户占比可能较高）。

---

## 8.5 Release 缺少 Python E2E 测试 🟡

**问题**: `release.yml` 的 `test` job 没跑 `tests/integration/test_e2e.py`。CI 中有 Python 单元测试（`python -m pytest tests/`），但 release 门控只跑了 Rust 和前端测试。

**涉及文件**:
- `.github/workflows/release.yml`

**建议**: 在 `test` job 中添加：
```yaml
- name: Setup Python
  uses: actions/setup-python@v5
  with:
    python-version: '3.12'

- name: Python tests
  run: |
    python -m pip install pytest
    python -m pytest tests/unit/
```

（E2E 测试需要 AgentPulse 运行中，release CI 环境不方便跑，但 unit tests 应该跑。）

---

## 8.6 缺少 `rust-toolchain.toml` 🟢

**问题**: CI 用 `dtolnay/rust-toolchain@stable`，本地开发无固定 toolchain 版本。不同开发者可能用不同 Rust 版本导致 CI 与本地行为不一致。

**涉及文件**:
- 新建 `apps/desktop/src-tauri/rust-toolchain.toml`

**建议**:
```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

---

## 8.7 根 `package.json` 残留依赖 🟢

**问题**: 根目录 `package.json` 含有 `vitest@^4.1.7`、`@vue/test-utils@^2.4.10`、`happy-dom@^20.9.0`，而实际项目在 `apps/desktop/` 下使用不同版本（`vitest@^2.1.8` 等）。根 `package.json` 的 devDependencies 似乎是残留，未被使用。

**涉及文件**:
- `package.json` (根目录)

**建议**: 清理根 `package.json` 中未使用的 devDependencies，或确认其用途后统一版本。
