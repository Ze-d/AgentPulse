# TODO: 接入真实 Claude Code 事件

## 状态

**已完成** — 2026-05-23

## 所做的工作

1. 重构 `install_hooks.py`：添加 `--dry-run`、`--status`、`--force`、幂等安装、自动备份
2. 重构 `monitor_hook.py`：添加 stderr 日志、3 次重试、可配置超时、`--test` 模式
3. 添加 30 个单元测试（22 install_hooks + 8 monitor_hook）
4. 运行 `install_hooks.py` 将 6 个 hook 事件注册到 `~/.claude/settings.json`
5. 用真实 Claude Code session 进行端到端验证

## 验证结果

- hooks 成功写入 `~/.claude/settings.json`，6 个事件全部注册
- AgentPulse 事件服务器正确接收真实 CC session 事件（已观测到 `tool_running` 状态的 session）
- E2E 测试通过（健康检查 + 5 事件 POST + session 验证）
- Python 单元测试 30/30 通过
- Rust 测试 25/25 通过
- TypeScript 类型检查零错误

## 相关文件

- [adapters/claude-code/install_hooks.py](../../adapters/claude-code/install_hooks.py)
- [adapters/claude-code/monitor_hook.py](../../adapters/claude-code/monitor_hook.py)
- [tests/unit/test_install_hooks.py](../../tests/unit/test_install_hooks.py)
- [tests/unit/test_monitor_hook.py](../../tests/unit/test_monitor_hook.py)
- [设计文档](../superpowers/specs/2026-05-23-connect-real-claude-code-design.md)
- [实现计划](../superpowers/plans/2026-05-23-connect-real-claude-code-plan.md)
