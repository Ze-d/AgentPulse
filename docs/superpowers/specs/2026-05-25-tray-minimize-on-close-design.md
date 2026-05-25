# Tray Minimize on Close

## Summary

When the user closes the AgentPulse window (X button / Alt+F4), intercept the close event and either minimize to tray or quit based on saved preference. On first close, show a native dialog asking the user to choose, with a "remember my choice" option.

## Files Changed

| File | Change |
|------|--------|
| `apps/desktop/src-tauri/src/lib.rs` | `on_close_requested` event handler + first-close dialog logic + preference persistence |
| `apps/desktop/src-tauri/src/tray.rs` | Rewrite: left-click restores from tray; Quit menu always exits regardless of preference |

## Behavior Flow

```
User clicks X / Alt+F4
        │
        ▼
  on_close_requested → prevent_close()
        │
        ▼
  Read close_action.json from app_data_dir
        │
   ┌────┴────┐
   │ exists?  │
   └────┬────┘
   yes       no
   │         │
   ▼         ▼
Follow      Show native dialog:
pref        ├─ "Minimize to Tray"
            ├─ "Quit"
            └─ "Remember my choice" checkbox
                 │
            User picks → if checkbox checked, write close_action.json
                 │
            Execute choice:
            - tray: window.hide()
            - quit: app.exit(0)
```

## Preference Storage

- Path: `{app_data_dir}/close_action.json`
- Format: `{"action": "tray"}` or `{"action": "quit"}`
- File absent = first run, prompt the dialog

## Tray Behavior

- Left-click tray icon: `window.show()` + `window.set_focus()` (if hidden), `window.hide()` (if visible)
- Tray menu "Quit": always calls `app.exit(0)`, ignores close preference

## Window Behavior

- `on_close_requested` prevents default close, routes to the flow above
- Window starts visible as before (`visible: true` in tauri.conf.json)
- All existing window props unchanged (transparent, alwaysOnTop, decorations: false)
