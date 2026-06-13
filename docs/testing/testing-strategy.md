# Testing Strategy

## Layers

| Layer | Tool | Scope |
|-------|------|-------|
| Rust unit | `cargo test` | db, state_machine, event_server, types, hooks, config (in-module tests) |
| Rust integration | `cargo test` | db integration (in-module `#[cfg(test)]`) |
| TypeScript type-check | `npx vue-tsc --noEmit` | Frontend type safety |
| Frontend unit | `npm test` (Vitest) | Components, composables, stores, utils |

## Running All Tests

```powershell
# Rust
cd apps/desktop/src-tauri
cargo test

# TypeScript
cd apps/desktop
npx vue-tsc --noEmit

# Frontend unit tests
cd apps/desktop
npm test
```
