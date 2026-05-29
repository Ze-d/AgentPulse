# Changelog

All notable changes to AgentPulse will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
