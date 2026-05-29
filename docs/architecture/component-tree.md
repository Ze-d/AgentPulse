# Component Tree

## Structure

```
App.vue
└── FloatingPanel.vue
    ├── Header
    │   ├── prompt (~/agentpulse $)
    │   ├── session count [N active]
    │   ├── refresh button (↻)
    │   └── minimize button (_)
    ├── Error banner            (dismissible, × close button)
    ├── Empty state             (loading: blinking _, loaded: "listening...")
    └── Session list
        ├── Transition (slide)
        └── SessionCard.vue          (collapsed state)
        │   ├── project name (sourceAbbr + tooltip)
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
| `App.vue` | Root mount point, initializes Pinia store |
| `FloatingPanel.vue` | Main panel: header with session count + refresh + minimize, dismissible error banner, loading/empty state, session list with slide transition. Polls store every 2s via `startPolling()` |
| `SessionCard.vue` | Single collapsed session card: source abbreviation via `sourceAbbr()`, project name with tooltip, formatted duration, status label, last tool name. `needsAttention` sessions get pulsing amber shadow animation |
| `ExpandedDetail.vue` | Expanded session view: full detail with open-directory / open-transcript action buttons via `@tauri-apps/plugin-opener` (`openPath`) |

## State Flow

```
sessionStore (Pinia)
  ├── sessions: AgentSession[]        ← invoke("get_sessions") every 2s
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

sourceDisplay (util)
  └── sourceAbbr(source)              ← "claude-code"→"cc", "codex"→"cx", etc.

openActions (util)
  ├── openDirectory(path)             ← openPath() for system file explorer
  └── openTranscript(path)            ← openPath() for default editor
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
