# Documentation Index

## 流程文档 (flows/)

- [流程文档索引](flows/README.md) — 全部流程的快速导航
- [01. 应用启动](flows/01-app-startup.md) — 从 main() 到窗口显示
- [02. Hooks 安装与配置](flows/02-hooks-installation.md) — 自动安装 + 手动安装
- [03. 事件捕获与转发](flows/03-event-capture.md) — monitor_hook.py: stdin → HTTP
- [04. 服务端事件处理](flows/04-event-processing.md) — HTTP 服务器 + 事件规范化
- [05. 状态机转换](flows/05-state-machine.md) — 7 种状态 × 8 种事件的转换规则
- [06. Session 生命周期](flows/06-session-lifecycle.md) — 从创建到清理的完整过程
- [07. 进程存活检测](flows/07-process-checker.md) — PID 获取 + 轮询清理
- [08. 前端轮询与 UI 渲染](flows/08-frontend-polling.md) — Pinia + 轮询 + 自适应窗口
- [09. 窗口关闭与托盘](flows/09-tray-close.md) — 关闭拦截 + 偏好记忆 + 托盘交互
- [10. 整体数据流总览](flows/10-data-flow-overview.md) — 端到端链路 + 时间线示例

## 架构与设计

- [架构概述](architecture/overview.md)
- [组件树](architecture/component-tree.md)
- [模块边界](architecture/module-boundaries.md)

## 测试

- [测试策略](testing/testing-strategy.md)
- [TDD 指南](testing/tdd-guide.md)
- [测试数据](testing/test-data.md)
- [代码规范](ai/coding-rules.md)
- [Context Map](ai/context-map.md)
- [审查清单](ai/review-checklist.md)

## 设计文档

- [设计文档 — AgentPulse v0.1](superpowers/specs/2026-05-22-agentpulse-v01-design.md)
- [设计文档 — 中文版](superpowers/specs/2026-05-22-agentpulse-v01-design-zh.md)
- [设计文档 — Connect Real CC](superpowers/specs/2026-05-23-connect-real-claude-code-design.md)
- [设计文档 — Terminal Pulse Style](superpowers/specs/2026-05-24-terminal-pulse-style-design.md)
- [设计文档 — Tray Minimize](superpowers/specs/2026-05-25-tray-minimize-on-close-design.md)
- [设计文档 — Session Persist](superpowers/specs/2026-05-25-session-persist-after-completion-design.md)
- [实现计划 — v0.1](superpowers/plans/2026-05-22-agentpulse-v01-plan.md)
- [实现计划 — Connect Real CC](superpowers/plans/2026-05-23-connect-real-claude-code-plan.md)
- [实现计划 — Terminal Pulse Style](superpowers/plans/2026-05-24-terminal-pulse-style-plan.md)
- [实现计划 — Tray Minimize](superpowers/plans/2026-05-25-tray-minimize-on-close.md)

## 未完成任务

- [代码质量改进](todos/03-code-quality.md) — 2/10 未完成
- [可访问性改进](todos/04-accessibility.md) — 0/7 未开始
- [测试与 CI 增强](todos/05-testing-and-ci.md) — 3/11 未完成
- [UX 增强改进](todos/06-ux-enhancements.md) — 5/5 已完成 ✅

## 修复记录

- [Bug 修复记录](fixlog/) — 7 篇修复日志

## 其他

- [发布流程](workflow/release.md)
- [本地开发指南](../local-development-guide.md)
