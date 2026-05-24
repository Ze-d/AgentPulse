# TODO: 修复页面无法拖拽的问题

## 状态

**已完成** — 2026-05-24

## 根因诊断（3 个层次）

### 根因 #1（致命）: 缺少 `core:window:allow-start-dragging` 权限

Tauri v2 默认禁用了窗口拖拽 API。`capabilities/default.json` 中只有 `core:default` 和 `opener:default`，缺少 `core:window:allow-start-dragging`。没有这个权限，`data-tauri-drag-region` 静默失败 — 不报错，也不生效。

### 根因 #2: 子元素拦截点击事件

`data-tauri-drag-region` 属性**不会自动传递给子元素**。`.panel-header` div 有这个属性，但当用户点击其子元素（`h1` 标题文字、`span` 计数）时，这些子元素是直接的点击目标，它们没有拖拽属性 → 拖拽不触发。只有点击文字之间微小的空隙才能拖拽。

### 根因 #3: 父级拖拽区域被遮蔽

`.floating-panel` 虽然有 `data-tauri-drag-region`，但其子元素通过 flexbox 填满了 100vh，父级裸露面积为零，拖拽区域实际上不可达。

## 修复内容

### 1. 添加 capability 权限

**文件**: [apps/desktop/src-tauri/capabilities/default.json](../../apps/desktop/src-tauri/capabilities/default.json)

添加 `core:window:allow-start-dragging` 到 permissions 列表。

### 2. 优化拖拽区域 (FloatingPanel.vue)

**文件**: [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)

- **移除** `.floating-panel` 上的 `data-tauri-drag-region`（父级被遮蔽，无效）
- **保留** `.panel-header` 上的 `data-tauri-drag-region`（主要拖拽区域）
- **添加** `.panel-header > * { pointer-events: none }` → 子元素文字透传点击事件给父级 div，使整条 header bar 都成为拖拽面
- **添加** `.empty-state` 上的 `data-tauri-drag-region`（无 session 时的替代拖拽区域）
- **添加** `.empty-state > * { pointer-events: none }` → 同样的子元素透传处理

## 验证结果

- Rust 编译: ✅ 通过 (`cargo check`)
- TypeScript 类型检查: ✅ 通过 (`vue-tsc --noEmit`)
- 功能验证: 需要启动 `tauri dev` 后手动确认拖拽是否正常
