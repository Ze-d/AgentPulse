# 9. 窗口关闭与托盘流程

## 涉及文件

- [lib.rs](../../apps/desktop/src-tauri/src/lib.rs) — 窗口关闭拦截逻辑
- [tray.rs](../../apps/desktop/src-tauri/src/tray.rs) — 系统托盘实现
- [commands.rs](../../apps/desktop/src-tauri/src/commands.rs) — `hide_main_window` 命令

## 概述

AgentPulse 的窗口关闭行为不同于普通应用：点 X 不是退出，而是最小化到系统托盘。首次关闭时询问用户偏好，后续记住选择。

## 窗口关闭拦截流程

```
用户点击窗口 X 按钮 / Alt+F4
  │
  └─→ tauri::WindowEvent::CloseRequested
        │
        ├─→ api.prevent_close()              // 阻止窗口立即关闭
        │
        ├─→ 读取 close_action.json
        │     │
        │     ├─→ 读取成功:
        │     │     ├─→ "tray" → 最小化到托盘
        │     │     └─→ "quit" → 直接退出
        │     │
        │     └─→ 读取失败 / 不存在:
        │           │
        │           ├─→ 弹出对话框1: "Minimize to system tray?"
        │           │     ├─→ Yes →
        │           │     │     ├─→ window.hide()              // 隐藏窗口
        │           │     │     ├─→ 弹出对话框2: "Always minimize to tray when closing?"
        │           │     │     │     ├─→ Yes → write_close_preference("tray")
        │           │     │     │     └─→ No  → 不保存
        │           │     │     └─→ 关闭对话框2
        │           │     │
        │           │     └─→ No  →
        │           │           ├─→ 弹出对话框2: "Always quit when closing?"
        │           │           │     ├─→ Yes → write_close_preference("quit")
        │           │           │     └─→ No  → 不保存
        │           │           └─→ app_handle.exit(0)          // 退出应用
        │           │
        │           └─→ (对话框是阻塞式的 blocking_show())
        │
        └─→ 结束
```

## 偏好存储

文件位置: `<app_data_dir>/close_action.json`

```json
{
  "action": "tray"
}
```

值:
- `"tray"` — 关闭时最小化到托盘
- `"quit"` — 关闭时直接退出
- 文件不存在 — 未设置，每次询问

## 托盘菜单交互

```
系统托盘图标
  │
  ├─→ 左键单击 → toggle 窗口显示/隐藏
  │     │
  │     ├─→ window.is_visible()?
  │     │     ├─→ true  → window.hide()
  │     │     └─→ false → window.show() + window.set_focus()
  │
  └─→ 右键菜单
        │
        ├─→ "Show / Hide" → 同左键单击
        │
        └─→ "Quit"
              └─→ app.exit(0)
                    └─→ 清理所有线程 + 退出进程
```

## 托盘初始化

```rust
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show / Hide", true, None)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(...)          // 右键菜单点击处理
        .on_tray_icon_event(...)     // 左键点击处理
        .build(app)?;
    Ok(())
}
```

## 前端关闭按钮

前端 FloatingPanel 头部的 `[_]` 按钮执行的是 `hide` 而非 `close`：

```typescript
// FloatingPanel.vue
async function handleClose() {
  await hideMainWindow();  // invoke("hide_main_window")
}

// commands.rs
#[tauri::command]
pub fn hide_main_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    let window = app_handle.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window.hide().map_err(|e| e.to_string())
}
```

这直接隐藏窗口，**不触发** `CloseRequested` 事件（因此不会弹出偏好询问对话框）。

## 完整状态机

```
                    应用启动
                       │
                       ▼
               ┌──────────────┐
               │ 窗口可见     │
               │ 托盘图标存在 │
               └──┬───┬───┬──┘
                  │   │   │
        点击 X    │   │   │  点击托盘 Quit
         (关闭)   │   │   │
                  │   │   └──────────────┐
                  ▼   │                  │
           读取偏好？  │ 托盘左键/Show    ▼
           /    \     │  Hide         ┌──────────┐
      tray      quit  │               │ 应用退出 │
       │         │    │               │ 进程终止 │
       ▼         ▼    │               └──────────┘
  ┌─────────┐ ┌──────┐│
  │窗口隐藏 │ │退出  ││
  │(托盘)   │ │应用  ││
  └────┬────┘ └──────┘│
       │               │
       └───────────────┘
      托盘左键/Show
```

## 关键设计决策

1. **阻塞式对话框** — `blocking_show()` 确保用户做出选择前窗口不会消失
2. **偏好持久化** — 写入文件系统，应用重启后保留
3. **前端 _ 按钮 = 隐藏而非关闭** — 直接调用 `hide_main_window`，跳过偏好检查
4. **窗口 X 按钮 = 关闭被拦截** — `api.prevent_close()` 阻止默认行为，执行自定义逻辑
5. **首次关闭时两次询问** — 第一个确认本次行为，第二个确认是否记住
