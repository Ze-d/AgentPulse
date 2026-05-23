use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn create_floating_window(app: &tauri::App) {
    let _win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("AgentPulse")
        .inner_size(320.0, 200.0)
        .min_inner_size(280.0, 120.0)
        .resizable(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(false)
        .visible(true)
        .build()
        .expect("Failed to create floating window");
}
