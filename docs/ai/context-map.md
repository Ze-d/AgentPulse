# Context Map

## Key Files

| File | Purpose |
|------|---------|
| `adapters/claude-code/install_hooks.py` | Install/uninstall/status/dry-run hooks in ~/.claude/settings.json |
| `adapters/claude-code/monitor_hook.py` | Read hook JSON from stdin, POST to event server :17878 |
| `apps/desktop/src-tauri/src/lib.rs` | Shared Rust types (AgentEvent, AgentSession, AgentStatus) |
| `apps/desktop/src-tauri/src/db.rs` | SQLite schema, CRUD for sessions and events |
| `apps/desktop/src-tauri/src/state_machine.rs` | Session status transition validation |
| `apps/desktop/src-tauri/src/event_server.rs` | HTTP server :17878, event normalization |
| `apps/desktop/src-tauri/src/commands.rs` | Tauri commands (get_sessions, get_session_detail) |
| `apps/desktop/src-tauri/src/tray.rs` | System tray with show/hide/quit |
| `apps/desktop/src/types/agent.ts` | TypeScript types matching Rust structs |
| `apps/desktop/src/stores/sessionStore.ts` | Pinia store: polling, expand/collapse |
| `apps/desktop/src/components/FloatingPanel.vue` | Main panel, session list, error banner |
| `apps/desktop/src/components/SessionCard.vue` | Single session card with status dot |
| `apps/desktop/src/components/ExpandedDetail.vue` | Expanded session detail with actions |
| `tests/unit/test_install_hooks.py` | Unit tests for install_hooks.py |
| `tests/unit/test_monitor_hook.py` | Unit tests for monitor_hook.py |
| `tests/integration/test_e2e.py` | E2E pipeline smoke test |
