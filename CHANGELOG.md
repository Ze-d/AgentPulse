# Changelog

All notable changes to AgentPulse will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
