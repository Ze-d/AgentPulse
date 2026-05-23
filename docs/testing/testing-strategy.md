# Testing Strategy

## Layers

| Layer | Tool | Scope |
|-------|------|-------|
| Rust unit | `cargo test` | db, state_machine, event_server, types (in-module tests) |
| Rust integration | `cargo test` | db integration (in-module `#[cfg(test)]`) |
| Python unit | `pytest tests/unit/` | install_hooks.py (22 cases), monitor_hook.py (8 cases) |
| Python E2E | `python tests/integration/test_e2e.py` | Full pipeline: POST events → verify DB state |
| TypeScript type-check | `npx vue-tsc --noEmit` | Frontend type safety |

## Running All Tests

```powershell
# Rust
cd apps/desktop/src-tauri
cargo test

# Python
python -m pytest tests/unit/ -v
python tests/integration/test_e2e.py

# TypeScript
cd apps/desktop
npx vue-tsc --noEmit
```
