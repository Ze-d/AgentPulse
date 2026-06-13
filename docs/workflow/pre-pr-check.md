# Pre-PR 检查清单

## 为什么需要 Pre-PR 检查

GitHub CI 在 **3 个平台**（Windows / Linux / macOS）上运行完整检查，单次耗时较长。如果在本地未验证就推送，容易出现：

- CI 在某个平台上报类型错误，本地却没问题
- Rust 格式化不一致导致 `cargo fmt --check` 失败
- 忘记运行测试，CI 上才发现测试挂了
- 依赖漏洞审计失败

**本地的几分钟检查 = 省去反复 push → CI 失败 → 修复 → push 的循环。**

---

## 快速开始

```powershell
# 默认模式（编译 + 测试 + lint，跳过审计）
.\scripts\ci-check.ps1

# 完整检查（含安全审计，推荐 PR 前最后一步）
.\scripts\ci-check.ps1 -Full
```

看到 `All checks passed! Safe to push/merge.` 即通过。

---

## 检查项详解

| # | 步骤 | 命令 | 作用 | 常见失败原因 |
|---|------|------|------|-------------|
| 1 | TypeScript 类型检查 | `npx vue-tsc --noEmit` | 确保类型系统无错误 | 接口变更未同步、第三方类型不匹配 |
| 2 | 前端测试 | `npm test` (Vitest) | 运行 Vue 组件 / 工具函数测试 | 测试用例未更新、快照过期 |
| 3 | Python 测试 | `python -m pytest tests/` | 运行 Python 后端测试 | 依赖缺失、接口不兼容 |
| 4 | Rust 格式 | `cargo fmt --check` | 代码风格一致性 | 忘了 `cargo fmt` |
| 5 | Rust Lint | `cargo clippy -- -D warnings` | 零警告策略 | 未使用的变量、不必要的 clone |
| 6 | Rust 测试 | `cargo test` | 运行 Tauri 后端测试 | 逻辑变更未更新测试 |
| 7 | Rust 审计 | `cargo audit` | 依赖漏洞扫描 | Rust 依赖有已知 CVE |
| 8 | npm 审计 | `npm audit --audit-level=high` | 高危漏洞扫描 | npm 依赖有已知漏洞 |

> 步骤 7、8 仅在 `-Full` 模式下运行。

---

## 逐项修复指南

### 1. vue-tsc 类型错误

```
error TS2322: Type '...' is not assignable to type '...'
```

- 先运行 `npx vue-tsc --noEmit` 确认本地可复现
- 检查最近修改的组件 props / emits 类型定义
- 如果第三方包类型有问题，检查 `package.json` 中版本是否锁定

### 2. 前端测试失败

```
FAIL  src/components/__tests__/xxx.test.ts
```

- 运行 `npm test -- --reporter=verbose` 查看详细输出
- 检查是否因 UI 改动导致快照过期：`npm test -- --update`
- 确认新增逻辑有对应的测试覆盖

### 3. Python 测试失败

```
FAILED tests/test_xxx.py::test_yyy
```

- 确认已安装依赖：`pip install -r requirements.txt`
- 检查函数签名或返回值是否变更

### 4. cargo fmt 格式问题

```
Diff in src-tauri/src/main.rs
```

- 运行 `cargo fmt` 自动修复后重新提交

### 5. cargo clippy 警告

```
warning: unused variable: `x`
error: could not compile `agent-pulse` (bin) due to 1 previous error
```

- Clippy 以 `-D warnings` 运行，任何警告都会变成错误
- 按警告信息逐条修复
- 如果是误报，用 `#[allow(clippy::xxx)]` 标注并加注释说明原因

### 6. Rust 测试失败

```
FAILED tests::test_xxx
```

- 运行 `cargo test -- --nocapture` 查看完整输出
- 检查是否有异步/时序相关的不稳定测试

### 7. cargo audit 漏洞

```
CVE-2024-XXXX: Vulnerability found in crate `xxx`
```

**处理优先级：**

| 等级 | 处理方式 |
|------|---------|
| Critical | 立即升级或寻找替代方案 |
| High | 优先升级，如无安全版本则评估风险 |
| Medium/Low | 评估后决定，记录风险评估 |

如果漏洞不影响本项目（如使用了未受影响的 API），可在 `.cargo/audit.toml` 中临时豁免，但必须加注释说明原因。

### 8. npm audit 漏洞

```
high severity vulnerability in package `xxx`
```

- 首先尝试 `npm audit fix`
- 如自动修复不完整，手动升级相关包
- 对于 `devDependencies` 中的低风险漏洞，评估后决定是否豁免

---

## CI 脚本封装了这些步骤

[`scripts/ci-check.ps1`](../../scripts/ci-check.ps1) 已经封装了上述所有步骤，参数说明：

| 参数 | 行为 |
|------|------|
| 无参数 | 类型检查 + 测试 + fmt + clippy（不含审计） |
| `-Full` | 包含 `cargo audit` + `npm audit` |
| `-Quick` | 仅类型检查 + 测试（跳过 fmt/clippy/审计） |

---

## 除了脚本检查，还需要确认

在 `ci-check.ps1` 通过后，PR 提交前还需人工确认以下事项：

### 代码质量

- [ ] 没有调试用的 `console.log` / `dbg!` / `print()` 残留
- [ ] 没有注释掉的大段代码（删除或移到独立分支）
- [ ] 变量 / 函数命名清晰，符合项目现有风格
- [ ] 新增公开 API 有对应的类型导出

### 安全问题

- [ ] 没有硬编码的密钥、token、密码
- [ ] 用户输入有适当的校验和清理
- [ ] 文件路径操作考虑了目录遍历风险
- [ ] 外部 API 调用有超时和错误处理

### 文档

- [ ] `CHANGELOG` 或 PR 描述说明了变更内容
- [ ] 如果接口有 breaking change，已标注
- [ ] 新增的配置项有说明

### Git

- [ ] 分支命名规范（`feat/`, `fix/`, `chore/` 等）
- [ ] commit 信息清晰，一个 commit 做一件事
- [ ] 已 rebase 到最新的 master，无冲突

---

## 推荐工作流

```powershell
# 1. 开发完成后，运行快速检查
.\scripts\ci-check.ps1

# 2. 修复所有问题后，运行完整检查
.\scripts\ci-check.ps1 -Full

# 3. 人工确认清单上的项目

# 4. 推送分支
git push origin feat/my-feature

# 5. 创建 PR
gh pr create --title "feat: xxx" --body "..."

# 6. 等待 GitHub CI 通过（3 平台）
#    如有失败，本地修复后 force push
```

---

## GitHub CI 对照

本地脚本对应 GitHub CI 配置 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 的 `check` job，两者检查项一一对应。差异点：

| 项 | 本地 | GitHub CI |
|----|------|-----------|
| 运行平台 | 仅 Windows | Windows + Linux + macOS |
| `npm ci` | ❌（使用已有 `node_modules`） | ✅（全新安装，防锁文件不一致） |
| Linux 系统依赖 | ❌（N/A） | ✅（`apt-get` 安装 GTK/WebKit 等） |
| cach | ❌ | ✅（缓存 Cargo 依赖，加速构建） |

---

## 常见问题

### Q: Windows 本地通过了，CI 在 Linux/macOS 上报错？

通常原因：
- 路径分隔符用 `\` 而非跨平台的 `path.join()` 或 `Path::new()`
- 文件名大小写问题（Windows 不区分大小写，Linux/macOS 区分）
- 依赖了仅 Windows 可用的库

排查方法：在 CI 日志中定位具体错误，优先怀疑文件路径和系统调用。

### Q: `npm audit` 一直报某个包有高危漏洞，但没有补丁？

评估该漏洞是否影响你的使用场景。如果确认不影响：
1. 在 `package.json` 的 `overrides` 中锁定安全版本（如果有）
2. 在 P 描述中说明风险评估结果
3. 可以暂时用 `npm audit --audit-level=critical` 降低门槛（不推荐）

### Q: CI 通过了但本地不过？

- 确认 Node.js / Rust / Python 版本与 CI 配置一致
- 运行 `npm ci` 替代 `npm install` 以匹配 CI 的依赖安装方式
- 清理缓存：`cargo clean && rm -rf node_modules && npm install`
