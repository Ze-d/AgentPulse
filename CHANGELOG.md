# Changelog

All notable changes to AgentPulse will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Swipe-to-dismiss for completed session cards (touch + mouse support)
  - New `useSwipeDismiss` composable with 80px threshold and spring-back animation
  - `dismissSession` store action with backend `delete_session` command
  - Visual feedback: "✕ dismiss" indicator with red background on threshold cross
- `delete_session` Tauri IPC command for manual session removal
- `skipTaskbar: true` in window config — floating panel no longer appears in taskbar
- Updated application icon (brand icon, 6 curated sizes)
- Tray icon explicit loading via `Image::from_bytes(include_bytes!(...))` for guaranteed icon embedding
- `image-ico` feature enabled for Tauri

### Changed
- `useSessionDisplay` now accepts `Ref<AgentSession>` instead of plain object — reactive prop changes now correctly trigger UI updates
- `STATUS_COLORS`: `completed` → `#a6e3a1` (Green), `running` → `#94e2d5` (Teal) — all 8 states now have unique colors
- Cleaned up icons directory: removed 11 unused StoreLogo/Android/iOS files (6 essential files remain)
- `tray.rs` now loads icon at compile time via `include_bytes!`

### Fixed
- **Reactivity Bug**: `SessionCard` not updating status display after polling refresh
  - Root cause: `useSessionDisplay` received non-reactive plain object, `computed` closure never re-evaluated
  - Fix: `toRef(props, "session")` → composable receives `Ref<AgentSession>` → `.value` access tracked by Vue

## [0.4.0] - 2026-06-07

### Added
- Configuration file support (`config.json`) with env var overrides
  - Configurable HTTP port, process check interval, Python interpreter, poll interval
  - Auto-generates default config on first launch
  - Environment variables as secondary overrides for CI/container use
- Structured logging via `tracing` (console + JSON file output with rotation)
- `get_config` Tauri command for frontend to read runtime config
- Detailed flow documentation (10 docs covering startup, hooks, events, sessions, etc.)
- Config module tests (5 unit tests)

### Changed
- `process_checker::start()` accepts configurable interval parameter
- `hooks::ensure_hooks_installed()` accepts Python interpreter parameter
- `resolve_python()` supports config hint + auto-detect fallback
- All todo files verified and updated with completion status (34/51 items done)

### Fixed
- CSP security policy in tauri.conf.json (was null, now strict)
- DB deserialize panics replaced with proper error types
- Event server lock poisoning handled gracefully
- process_pid now persists correctly through DB round-trip
- Event server error logging and graceful shutdown

## [0.3.0] - 2026-05-29

### Added
- Cross-platform CI matrix (Windows, Ubuntu, macOS)
- CI lint gates: `cargo fmt --check` + `cargo clippy -- -D warnings`
- CI security audit: `cargo audit` + `npm audit`
- Python test suite integration in CI (pytest, 32 tests)
- Frontend test infrastructure (Vitest + @vue/test-utils + happy-dom, 18 tests)
- `npm test` / `npm run test:watch` scripts
- Release workflow test gate (fmt + clippy + test before build)
- Reusable `useSessionDisplay` composable for session display logic

### Changed
- CI uses `npm ci` instead of `npm install` for reproducible builds
- Release workflow upgraded to `tauri-apps/tauri-action@v2`
- DB row mapping extracted to `map_session_row()` helper (eliminated ~60 lines of duplication)
- CSS hardcoded values extracted as module-level constants in `FloatingPanel.vue`
- IPC calls wrapped in typed functions (`getSessions()`, `hideMainWindow()`)
- Consolidated CSS reset styles to `main.css`

### Removed
- Unused Tailwind CSS import (no utility classes were in use)
- Dead code: `vue.svg`, unused store state/getters/actions
- Redundant integration tests (coverage preserved by richer module tests)
- Production `console.debug` log leakage

### Fixed
- Clippy warnings: `redundant_closure` and `new_without_default`

## [0.2.2]

### Added
- Session lifecycle management with floating panel UI
- Multi-source agent support (Claude Code, Codex, Gemini, Copilot)
- Process tree walking for accurate PID tracking on Windows
- System tray integration with minimized window support
- Transcript viewer and project directory opener
- Comprehensive documentation and accessibility improvements

### Fixed
- CSP configuration for Tauri webview security
- Database panic on corrupt enum values
- Lock poisoning in state machine
- PID loss on process exit detection
- Error logging and graceful shutdown handling

## [0.1.0] - 2026-05-01

### Added
- Initial release of AgentPulse desktop monitor
- Real-time session monitoring via Claude Code hooks
- SQLite-backed session and event storage
- Floating overlay panel with session cards
- Event server (HTTP) for receiving monitor hook payloads
- State machine for session lifecycle tracking
