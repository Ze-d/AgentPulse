# 1. 应用启动流程

## 涉及文件

- [main.rs](../../apps/desktop/src-tauri/src/main.rs) — 二进制入口
- [lib.rs](../../apps/desktop/src-tauri/src/lib.rs) — `run()` 函数，应用初始化核心
- [hooks.rs](../../apps/desktop/src-tauri/src/hooks.rs) — Hooks 安装逻辑
- [event_server.rs](../../apps/desktop/src-tauri/src/event_server.rs) — HTTP 事件服务器
- [process_checker.rs](../../apps/desktop/src-tauri/src/process_checker.rs) — 进程存活检测
- [tray.rs](../../apps/desktop/src-tauri/src/tray.rs) — 系统托盘

## 流程图

```
main()
  │
  └─→ lib::run()
        │
        ├─→ env_logger::init()                           // 初始化日志
        │
        ├─→ Database::new_in_memory()                    // 创建内存 SQLite 数据库
        │     └─→ init_schema()                          // 建表 (sessions, events)
        │
        ├─→ Arc<Mutex<Database>>                         // 共享数据库引用
        │
        ├─→ EventServer::start_shared(db, "127.0.0.1:17878")  // 启动 HTTP 服务器（独立线程）
        │     └─→ tiny_http::Server::http()               // 绑定 :17878
        │     └─→ thread::spawn(run_server_loop)          // 在独立线程中处理请求
        │
        ├─→ process_checker::start(db)                   // 启动进程存活检测（独立线程）
        │     └─→ thread::spawn(loop { sleep(5s); check })   // 每 5 秒检测
        │
        └─→ tauri::Builder::default()
              │
              ├─→ .plugin(tauri_plugin_opener)           // 注册 opener 插件（打开文件/目录）
              ├─→ .plugin(tauri_plugin_shell)            // 注册 shell 插件
              ├─→ .plugin(tauri_plugin_dialog)           // 注册 dialog 插件（消息框）
              │
              ├─→ .manage(AppState { db })               // 注入共享数据库到 Tauri State
              │
              ├─→ .invoke_handler(commands)              // 注册 IPC 命令（7 个）
              │
              └─→ .setup(|app| {                         // 应用就绪回调
                    │
                    ├─→ tray::setup_tray(app)            // 创建系统托盘
                    │     ├─→ MenuItem("Show / Hide")
                    │     ├─→ MenuItem("Quit")
                    │     ├─→ on_menu_event()            // 右键菜单事件
                    │     └─→ on_tray_icon_event()       // 左键点击（显示/隐藏窗口）
                    │
                    ├─→ window.on_window_event(CloseRequested)  // 拦截窗口关闭事件
                    │     └─→ 处理关闭偏好（托盘/退出/询问）    // 详见 09-tray-close.md
                    │
                    └─→ thread::spawn(ensure_hooks_installed)  // 启动后自动注册 hooks（独立线程）
                          ├─→ 获取 resource_dir（monitor_hook.py 位置）
                          ├─→ 获取 app_data_dir（提取目标位置）
                          ├─→ resolve ~/.claude/settings.json 路径
                          ├─→ extract_monitor_script()          // 复制 monitor_hook.py 到 app_data_dir
                          └─→ ensure_hooks_installed()          // 向 settings.json 写入 6 个 hook
                    })
```

## 关键时序说明

1. **数据库最先初始化** — 在启动 HTTP 服务器和进程检测器之前，确保数据层就绪
2. **HTTP 服务器在独立线程** — `thread::spawn`，不阻塞 Tauri 主线程
3. **进程检测器在独立线程** — 启动后立即进入 5 秒循环
4. **Hooks 自动安装异步执行** — 在 `setup` 闭包中的独立线程，不阻塞窗口创建
5. **所有线程共享同一个 `Arc<Mutex<Database>>`** — 通过 Tauri State 和 clone 传递

## 模块初始化顺序

```
时间线 ─────────────────────────────────────────────────────→

1. env_logger 初始化
2. Database 创建 + Schema 初始化
3. EventServer 线程启动 (端口 17878)
4. ProcessChecker 线程启动 (5s 循环)
5. Tauri Builder 开始
   ├─ 插件注册 (opener, shell, dialog)
   ├─ State 注入
   ├─ IPC 命令注册
   └─ setup 回调
       ├─ 托盘创建
       ├─ 窗口关闭拦截
       └─ Hooks 自动安装（异步线程）
6. 窗口显示
```

## 错误处理策略

- `Database::new_in_memory()` 失败 → `expect` panic（不可恢复，没有数据库应用无意义）
- HTTP 服务器绑定失败 → 错误日志 + 终止线程（应用继续运行但无法接收事件）
- Hooks 安装失败 → 仅记录错误日志，不阻止应用启动（用户可手动安装）
- Tray 创建失败 → `setup` 返回 `Err` 传播给 Tauri
