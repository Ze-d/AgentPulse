# BugFix: 无边框窗口无法拖拽移动

**日期:** 2026-05-24
**严重程度:** 高（核心交互不可用）
**状态:** 已修复

## 现象

AgentPulse 悬浮窗无法通过鼠标拖拽移动。窗口配置为 `decorations: false`（无边框），已使用 Tauri v2 的 `data-tauri-drag-region` 属性在 `.panel-header` 和 `.floating-panel` 上，但拖拽完全无效。

## 排查过程

1. **审查窗口配置** — `tauri.conf.json` 中 `decorations: false` + `alwaysOnTop: true`，确认是无边框窗口，需要自定义拖拽
2. **审查 HTML/CSS** — `FloatingPanel.vue` 已在 `.floating-panel` 和 `.panel-header` 上设置 `data-tauri-drag-region`，`.panel-header` 有 `cursor: grab`
3. **检查 `data-tauri-drag-region` 文档** — 搜索 Tauri v2 已知问题和 CSDN 避坑指南，发现常见失败原因
4. **检查权限配置** — `capabilities/default.json` 中缺少 `core:window:allow-start-dragging` 权限 ← **致命根因**

## 根因分析

**致命根因: 缺少 `core:window:allow-start-dragging` 权限**

Tauri v2 默认禁用了窗口拖拽 API。`data-tauri-drag-region` 需要 `core:window:allow-start-dragging` 权限才能生效。在 [capabilities/default.json](../../apps/desktop/src-tauri/capabilities/default.json) 中只有 `core:default` 和 `opener:default`，没有这个权限 → `data-tauri-drag-region` 静默失效，无任何错误提示。

**次要根因 #1: 子元素拦截点击事件**

`data-tauri-drag-region` 属性不会自动传递给子元素。`.panel-header` div 有此属性，但用户点击的是其子元素（`h1` 标题、`span` 计数），这些子元素是直接点击目标且无拖拽属性 → 只有点击文字间的微隙才能触。有效拖拽面积 ≈ 几个像素。

**次要根因 #2: 父级拖拽区域被完全遮蔽**

`.floating-panel` 虽有 `data-tauri-drag-region`，但 flex children 填满 100vh，父级裸露面积为零，拖拽区域不可达。

## 修复

### 1. 添加 capability 权限

**文件**: [apps/desktop/src-tauri/capabilities/default.json](../../apps/desktop/src-tauri/capabilities/default.json)

```diff
  "permissions": [
    "core:default",
+   "core:window:allow-start-dragging",
    "opener:default"
  ]
```

### 2. 优化 FloatingPanel.vue 拖拽区域

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)

- 移除 `.floating-panel` 的无效果 `data-tauri-drag-region`
- 保留 `.panel-header` 的 `data-tauri-drag-region` 作为主拖拽手柄
- 添加 `.panel-header > * { pointer-events: none }` → 子元素文字透传点击事件给父级，整条 header bar 成为拖拽面
- `.empty-state` 添加 `data-tauri-drag-region` 作为无 session 时的替代拖拽区域
- `.empty-state > *` 同样添加 `pointer-events: none`

## 验证方式

```powershell
# Rust 编译
cd apps/desktop/src-tauri && cargo check
# → Finished `dev` profile

# TypeScript 类型检查
cd apps/desktop && npx vue-tsc --noEmit
# → 零错误

# 功能验证（需要启动 tauri dev）
# 1. 启动 AgentPulse: cd apps/desktop && npm run tauri dev
# 2. 用鼠标拖拽 panel-header 区域 → 窗口应可移动
# 3. 在 empty-state 区域拖拽 → 窗口应可移动
```

## 相关文件

- [apps/desktop/src-tauri/capabilities/default.json](../../apps/desktop/src-tauri/capabilities/default.json) — 权限配置
- [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue) — 拖拽区域 HTML/CSS
- [docs/local-development-guide.md](../../docs/local-development-guide.md) — 项目结构文档（window.rs 引用已清理）
