# Tray Minimize on Close Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Intercept window close to minimize to tray by default, with a first-run dialog asking "Minimize to tray or quit?" and an option to remember the choice.

**Architecture:** Add `tauri-plugin-dialog` dependency, rewrite `tray.rs` for clean tray behavior, and add a `on_window_event(CloseRequested)` handler in `lib.rs` that reads/writes a JSON preference file.

**Tech Stack:** Tauri v2 (Rust), tauri-plugin-dialog v2

---

### Task 1: Add tauri-plugin-dialog dependency

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`

- [ ] **Step 1: Add dialog plugin to Cargo.toml**

Add `tauri-plugin-dialog` under `[dependencies]`, right after the existing `tauri-plugin-shell` line:

```toml
tauri-plugin-dialog = "2"
```

- [ ] **Step 2: Build to verify dependency resolves**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: dependency resolves, no errors (unused import warnings are OK at this stage)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.lock
git commit -m "feat: add tauri-plugin-dialog dependency"
```

---

### Task 2: Rewrite tray.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/tray.rs`

- [ ] **Step 1: Replace tray.rs content**

```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Create the system tray icon with Show/Hide and Quit menu items.
/// Left-click toggles window visibility. Quit always exits immediately.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

This is functionally identical to the current implementation — no logic changes needed in tray.rs because the current behavior is already correct for the new close flow. The tray "Quit" always calls `app.exit(0)` regardless of close preference, and left-click toggles show/hide.

- [ ] **Step 2: Build check**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src-tauri/src/tray.rs
git commit -m "refactor: clean up tray.rs doc comment"
```

---

### Task 3: Add close preference persistence and window close interception

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Read current lib.rs to confirm exact content**

File: `apps/desktop/src-tauri/src/lib.rs` — the `run()` function in its current form is the target.

- [ ] **Step 2: Add new imports at the top of lib.rs**

Insert after `use tauri::Manager;` (line 10):

```rust
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use serde::{Deserialize, Serialize};
```

And add the preference struct after the existing `AgentSession` struct (after line 83):

```rust
#[derive(Debug, Serialize, Deserialize)]
struct ClosePreference {
    action: String,
}
```

- [ ] **Step 3: Add helper functions before `pub fn run()`**

Insert the following two helper functions right before `pub fn run()` (before line 86):

```rust
fn read_close_preference(path: &std::path::PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<ClosePreference>(&s).ok())
        .map(|p| p.action)
}

fn write_close_preference(path: &std::path::PathBuf, action: &str) {
    let pref = ClosePreference {
        action: action.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&pref) {
        let _ = std::fs::write(path, json);
    }
}
```

- [ ] **Step 4: Register tauri-plugin-dialog in the builder**

In the `run()` function, add `.plugin(tauri_plugin_dialog::init())` after the existing shell plugin line. Change:

```rust
.plugin(tauri_plugin_opener::init())
.plugin(tauri_plugin_shell::init())
```

To:

```rust
.plugin(tauri_plugin_opener::init())
.plugin(tauri_plugin_shell::init())
.plugin(tauri_plugin_dialog::init())
```

- [ ] **Step 5: Add on_window_event handler inside `.setup()`**

Inside the `.setup(|app| { ... })` closure, right after `tray::setup_tray(app)?;` (currently line 110), insert the close event handler:

```rust
tray::setup_tray(app)?;

// Intercept window close to minimize to tray (with remembered preference)
let window = app.get_webview_window("main").unwrap();
let app_handle = app.handle().clone();
window.on_window_event(move |event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();

        let pref_path = app_handle
            .path()
            .app_data_dir()
            .unwrap()
            .join("close_action.json");

        let action = read_close_preference(&pref_path);

        match action.as_deref() {
            Some("tray") => {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Some("quit") => {
                app_handle.exit(0);
            }
            _ => {
                // No saved preference — ask the user
                let minimize = app_handle
                    .dialog()
                    .message("Minimize to system tray?")
                    .title("AgentPulse")
                    .kind(MessageDialogKind::Question)
                    .buttons(MessageDialogButtons::YesNo)
                    .blocking_show();

                if minimize {
                    // User chose tray — hide window
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }

                    let remember = app_handle
                        .dialog()
                        .message("Always minimize to tray when closing?")
                        .title("AgentPulse")
                        .kind(MessageDialogKind::Question)
                        .buttons(MessageDialogButtons::YesNo)
                        .blocking_show();

                    if remember {
                        write_close_preference(&pref_path, "tray");
                    }
                } else {
                    // User chose quit — exit app
                    let remember = app_handle
                        .dialog()
                        .message("Always quit when closing?")
                        .title("AgentPulse")
                        .kind(MessageDialogKind::Question)
                        .buttons(MessageDialogButtons::YesNo)
                        .blocking_show();

                    if remember {
                        write_close_preference(&pref_path, "quit");
                    }

                    app_handle.exit(0);
                }
            }
        }
    }
});

// Ensure hooks are installed on every launch (idempotent).
```

- [ ] **Step 6: Build check**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: no errors

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs
git commit -m "feat: intercept window close, minimize to tray with remembered preference"
```

---

### Task 4: Build and verify

**Files:** None (verification only)

- [ ] **Step 1: Full build**

Run: `cd apps/desktop/src-tauri && cargo build`
Expected: build succeeds with no errors

- [ ] **Step 2: Run clippy**

Run: `cd apps/desktop/src-tauri && cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Manual smoke test checklist**

Launch the app and verify:
1. First close → dialog appears with "Minimize to system tray?" Yes/No
2. Click "Yes" → window hides to tray, second dialog "Always minimize to tray when closing?"
3. Click "Yes" → preference saved
4. Close again → window hides directly to tray (no dialog)
5. Tray icon left-click → window shows again
6. Tray menu → Quit → app exits
7. Delete `close_action.json` from app data dir
8. Repeat test but choose "No" (quit), then "Yes" to remember
9. Close again → app exits directly (no dialog)

- [ ] **Step 4: Commit if any fixes needed**

Only if changes were needed during verification.
