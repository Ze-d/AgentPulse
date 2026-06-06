# Component Tree

## Structure

```
App.vue
└── FloatingPanel.vue
    ├── Header (data-tauri-drag-region)
    │   ├── prompt (~/agentpulse $)
    │   ├── session count [N active]
    │   ├── refresh button (↻)
    │   └── minimize button (_)
    ├── Error banner            (dismissible, × close button)
    ├── Empty state             (loading: blinking _, loaded: "agentpulse is listening...")
    └── Session list
        ├── Transition (slide)
        └── SessionCard.vue          (collapsed state)
        │   ├── source-abbr > project-name (tooltip)
        │   ├── duration            (live-updating)
        │   ├── status label
        │   ├── last tool name
        │   └── attention pulse     (needsAttention animation)
        └── ExpandedDetail.vue       (expanded state)
            ├── status + duration
            ├── cwd (working directory)
            ├── last tool name
            ├── transcript path
            ├── last message
            └── action buttons (open dir / open transcript)
```

## Component Responsibilities

| Component | Responsibility |
|-----------|---------------|
| `App.vue` | Root mount point, renders FloatingPanel |
| `FloatingPanel.vue` | Main panel: header with session count + refresh + minimize, dismissible error banner, loading/empty state, session list with slide transition. Fetches config via `getConfig()` on mount, starts polling with configurable interval |
| `SessionCard.vue` | Single collapsed session card: source abbreviation via `sourceAbbr()`, project name with tooltip, formatted duration via `useSessionDisplay()`, status label. `needsAttention` sessions get pulsing amber shadow animation |
| `ExpandedDetail.vue` | Expanded session view: full detail grid with status, duration, cwd, tool, transcript, message. Action buttons for open-directory / open-transcript via `@tauri-apps/plugin-opener` |

## State Flow

```
sessionStore (Pinia)
  ├── sessions: AgentSession[]        ← invoke("get_sessions") every N ms (configurable, default 2000)
  ├── expandedSessionId: string|null  ← toggleExpand()
  ├── error: string|null              ← catch block from invoke / clearError()
  ├── isLoading: boolean              ← true until first fetch completes
  └── actions:
      ├── fetchSessions()             ← get_sessions + finally { isLoading = false }
      ├── clearError()                ← set error = null
      └── toggleExpand(id)            ← toggle expandedSessionId
  └── getters:
      ├── activeSessions              ← filter out completed, failed
      ├── attentionSessions           ← filter needsAttention
      └── expandedSession             ← find(expandedSessionId)

useSessionDisplay (composable)
  ├── statusColor(session)            ← STATUS_COLORS lookup
  ├── statusLabel(session)            ← STATUS_LABELS lookup
  └── duration(session)               ← formatDuration(startedAt, completedAt)

sourceDisplay (util)
  └── sourceAbbr(source)              ← "claude-code"→"cc", "codex"→"cx", "gemini"→"gm", "copilot"→"cp"

ipc (util)
  ├── getSessions()                   ← invoke("get_sessions")
  ├── getConfig()                     ← invoke("get_config") → { pollIntervalMs }
  └── hideMainWindow()               ← invoke("hide_main_window")

openActions (util)
  ├── openDirectory(path)             ← openPath() for system file explorer
  └── openTranscript(path)            ← openPath() for default editor

logger (util)
  └── createLogger(module)            ← 条件日志 + Tauri log_event 转发
```

## Status Color Mapping

| Status | Color |
|--------|-------|
| starting | yellow |
| running | green |
| tool_running | blue |
| waiting_input | yellow |
| waiting_permission | red |
| completed | gray |
| failed | red |
| unknown | gray |
