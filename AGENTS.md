# AGENTS.md

## Project: AgentPulse

## Instructions

- Write tests first (TDD), verify they fail, then implement
- Run tests before committing
- Follow existing code patterns in the project
- Keep changes minimal and focused
- 使用中文回答用户，代码注释使用英文

## Test Commands

```powershell
# Python unit tests
python -m pytest tests/unit/ -v

# Python E2E test (requires AgentPulse running)
python tests/integration/test_e2e.py

# Rust tests
cd apps/desktop/src-tauri && cargo test

# TypeScript type check
cd apps/desktop && npx vue-tsc --noEmit
```

## Key Files

| File | Purpose |
|------|---------|
| `adapters/claude-code/install_hooks.py` | Hook install/uninstall/status CLI |
| `adapters/claude-code/monitor_hook.py` | Hook event stdin→HTTP adapter |
| `apps/desktop/src-tauri/src/lib.rs` | Shared types + app entry point |
| `apps/desktop/src-tauri/src/db.rs` | SQLite database |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP event server :17878 |
| `apps/desktop/src-tauri/src/state_machine.rs` | State transitions |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri IPC commands |
| `apps/desktop/src/stores/sessionStore.ts` | Frontend state + polling |
| `apps/desktop/src/components/FloatingPanel.vue` | Main floating panel |
