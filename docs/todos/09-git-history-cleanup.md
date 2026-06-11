# 09 — Git 历史整理

**状态：** 待执行
**优先级：** 中
**创建日期：** 2026-06-10

---

## 9.1 整理 master 提交历史 🟡

**问题**: 当前 master 分支包含大量细碎提交，同一功能拆成多条、类型混杂。例如 "Python-free hook adapter" 相关有 5 条独立提交（2 docs + 2 feat + 1 docs），应合并为 1 条 `feat:`。

**目标**: master 历史线性、一条提交对应一个功能/修复/发布，方便 `git bisect`、review 和 changelog 生成。

### 待 squash 的提交组（初步）

| 合并后提交 | 包含的原始提交 |
|-----------|---------------|
| `feat: replace Python hook adapter with Rust binary` | `ba69be1`, `5b27974`, `2eaeaa2`, `1e5bb39`, `461ef13` |
| `feat: add Codex integration` | `5f5e2a8`, `4805920`, `8733438`, `e855f87`, `19c0f25`, `d3d786f`, `4cc0e34`, `d5bd2ca`, `17f14aa`, `4c1bd19` |
| `fix: auto-detect agent source and session routing` | `b583065`, `4cc0e34`, `d5bd2ca` (需确认归属) |
| `feat: swipe-to-dismiss and session deletion` | `cdd8429`, `b6e83af` |
| `feat: configuration and structured logging` | `d526824`, `043d599`, `55da4e9` |

### 执行方式

```powershell
# 1. 从 master 创建整理分支
git checkout master
git checkout -b chore/cleanup-git-history

# 2. Interactive rebase
git rebase -i <第一个要保留的commit>~1

# 3. 在编辑器中标记 squash/fixup
# pick c570329 → 保留
# pick dcda098 → squash 到上一个
# ...

# 4. 检查结果
git log --oneline --graph

# 5. 强制推送（⚠️ 需要团队知情）
git push origin master --force-with-lease
```

### ⚠️ 风险

- `--force-with-lease` 会重写远程历史，协作者需要 `git pull --rebase` 或重新 clone
- 如果有基于当前 master 的未合入 PR，rebase 后可能需要重新基于新 master
- 建议在确认无未合入 PR 时执行

### 执行时机

- 下一个 release 之前
- 确认无未合入的 feature branch

---

## 9.2 修复异常 commit 格式 🟢

**问题**: `354ab53` commit message 为 `@ fix: resolve 7 high-priority bugs ...`，多余的 `@` 符号。

**改动**: rebase 时将 message 改为 `fix: resolve 7 high-priority bugs ...`

---

## 9.3 补充缺少前缀的 commit 🟢

**问题**: `94c8094` 缺少 conventional commit 前缀。

**改动**: rebase 时改为 `docs: add detailed flow documentation for AgentPulse features`
