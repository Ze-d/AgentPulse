# TODO: 优化页面样式

## 状态

**已完成** — 2026-05-24

## 所做的工作

Terminal Pulse 风格改造，将浮动面板从通用暗色 UI 改造为终端/CLI 风格工具伴侣。

### 排版改造
- 全局等宽字体栈: `Cascadia Code > JetBrains Mono > Fira Code > Consolas > monospace`
- 标题改为 CLI prompt 风格: `~/agentpulse $`
- 状态用彩色文字替代圆点: running/tool/waiting/done/failed
- 卡片改为终端行式布局: `cc > project-name  1m  running`

### 布局改造
- 面板高度自适应: 空状态 64px，根据 session 数量自动扩展，最大 420px
- Tauri Window API 动态调整窗口大小
- 间距收紧: 14px padding, 6-8px margin

### 白边修复
- 窗口 `transparent: true`，四角透明无白边

### 颜色
- 保持 Catppuccin Mocha 配色方案不变

## 实现记录

- 设计文档: [docs/superpowers/specs/2026-05-24-terminal-pulse-style-design.md](../superpowers/specs/2026-05-24-terminal-pulse-style-design.md)
- 实现计划: [docs/superpowers/plans/2026-05-24-terminal-pulse-style-plan.md](../superpowers/plans/2026-05-24-terminal-pulse-style-plan.md)
- 提交: 10 commits (709dcc6 ~ ec7c29e)

## 相关文件

- [apps/desktop/src/assets/main.css](../../apps/desktop/src/assets/main.css) — 全局字体 + 透明背景
- [apps/desktop/src/App.vue](../../apps/desktop/src/App.vue) — 透明背景
- [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue) — CLI header + 自适应高度
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue) — 终端行式卡片
- [apps/desktop/src/components/ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue) — 终端风格详情
- [apps/desktop/src/types/agent.ts](../../apps/desktop/src/types/agent.ts) — 单字状态标签
- [apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json) — 透明窗口配置
- [apps/desktop/src-tauri/capabilities/default.json](../../apps/desktop/src-tauri/capabilities/default.json) — 窗口权限
