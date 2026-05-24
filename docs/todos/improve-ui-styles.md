# TODO: 优化页面样式

## 状态

**待处理**

## 问题描述

需要改进和优化 AgentPulse 浮动面板的视觉样式，提升整体 UI 质感。

## 当前样式概况

- **配色方案**: Catppuccin Mocha 暗色主题（base `#1e1e2e`，surface0 `#313244`，text `#cdd6f4`）
- **窗口尺寸**: 320×200 浮动面板，无边框，始终置顶
- **圆角**: 12px 整体圆角，8px 卡片圆角
- **布局**: 单列垂直布局，header + session list

## 涉及文件

- [apps/desktop/src/assets/main.css](../../apps/desktop/src/assets/main.css) — CSS 变量、全局样式、Tailwind
- [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue) — 面板容器布局
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue) — 会话卡片样式
- [apps/desktop/src/components/ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue) — 展开详情样式
- [apps/desktop/src/App.vue](../../apps/desktop/src/App.vue) — 根组件样式

## 待明确

- 具体需要修改哪些样式？（字体、间距、颜色、动画、布局等）
- 是否要调整窗口尺寸？
- 是否需要支持主题切换？
