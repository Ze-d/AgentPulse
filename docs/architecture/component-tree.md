# Component Tree

## Structure

```
App.vue
└── FloatingPanel.vue
    ├── SessionCard.vue          (collapsed state)
    │   ├── status dot          (color-coded by AgentStatus)
    │   ├── project name
    │   ├── duration            (live-updating)
    │   └── last tool name
    ├── ExpandedDetail.vue       (expanded state)
    │   ├── status + duration
    │   ├── cwd (working directory)
    │   ├── last tool name
    │   ├── transcript path
    │   ├── last message
    │   └── action buttons (open dir / open transcript)
    └── Error banner            (conditional)
```

## Component Responsibilities

| Component | Responsibility |
|-----------|---------------|
| `App.vue` | Root mount point, initializes Pinia store |
| `FloatingPanel.vue` | Main panel: header with active count, session list, error banner, empty state. Polls store every 2s via `startPolling()` |
| `SessionCard.vue` | Single collapsed session card: status color dot, project name, formatted duration, last tool name. `needsAttention` sessions get pulse border animation |
| `ExpandedDetail.vue` | Expanded session view: full detail with open-directory / open-transcript action buttons via `@tauri-apps/plugin-shell` |

## State Flow

```
sessionStore (Pinia)
  ├── sessions: AgentSession[]        ← invoke("get_sessions") every 2s
  ├── expandedSessionId: string|null  ← toggleExpand()
  ├── error: string|null              ← catch block from invoke
  └── getters:
      ├── activeSessions              ← filter(completed, failed)
      ├── attentionSessions           ← filter(needsAttention)
      └── expandedSession             ← find(expandedSessionId)
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
