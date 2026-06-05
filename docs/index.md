# Documentation Index

## 流程文档 (flows/)

- [流程文档索引](flows/README.md) — 全部流程的快速导航
- [01. 应用启动](flows/01-app-startup.md) — 从 main() 到窗口显示
- [02. Hooks 安装与配置](flows/02-hooks-installation.md) — 自动安装 + 手动安装
- [03. 事件捕获与转发](flows/03-event-capture.md) — monitor_hook.py: stdin → HTTP
- [04. 服务端事件处理](flows/04-event-processing.md) — HTTP 服务器 + 事件规范化
- [05. 状态机转换](flows/05-state-machine.md) — 7 种状态 × 8 种事件的转换规则
- [06. Session 生命周期](flows/06-session-lifecycle.md) — 从创建到清理的完整过程
- [07. 进程存活检测](flows/07-process-checker.md) — PID 获取 + 5s 轮询清理
- [08. 前端轮询与 UI 渲染](flows/08-frontend-polling.md) — Pinia + 2s 轮询 + 自适应窗口
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
- [本地开发指南](../local-development-guide.md) (docs/ 同级)
- [设计文档 — AgentPulse v0.1](superpowers/specs/2026-05-22-agentpulse-v01-design.md)
- [设计文档 — Connect Real CC](superpowers/specs/2026-05-23-connect-real-claude-code-design.md)
- [实现计划 — v0.1](superpowers/plans/2026-05-22-agentpulse-v01-plan.md)
- [实现计划 — Connect Real CC](superpowers/plans/2026-05-23-connect-real-claude-code-plan.md)
- [接入真实 Claude Code 事件](todos/connect-real-claude-code.md)
- [Bug 修复记录](fixlog/)
