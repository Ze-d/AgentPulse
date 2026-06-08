# 1. 应用启动流程

## 涉及文件

- [main.rs](../../apps/desktop/src-tauri/src/main.rs) — 二进制入口
- [lib.rs](../../apps/desktop/src-tauri/src/lib.rs) — `run()` 函数，应用初始化核心
- [config.rs](../../apps/desktop/src-tauri/src/config.rs) — 配置加载（config.json + 环境变量覆盖）
- [logging.rs](../../apps/desktop/src-tauri/src/logging.rs) — tracing 日志初始化
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
        ├─→ logging::default_app_data_dir()              // 确定 app_data_dir
        │     ├─ Windows: %APPDATA%\com.agentpulse.desktop
        │     ├─ macOS:   ~/Library/Application Support/com.agentpulse.desktop
        │     └─ Linux:   $XDG_DATA_HOME/com.agentpulse.desktop
        │
        ├─→ logging::init(&log_dir)                      // 初始化 tracing
        │     ├─ stderr 层: 紧凑可读文本 + ANSI 颜色（debug 构建）
        │     ├─ file 层: JSON 结构化 + 每小时轮转
        │     ├─ 清理 7 天前的旧日志文件
        │     └─ 返回 WorkerGuard（必须保持存活）
        │
        ├─→ AgentPulseConfig::load(&app_data_dir)         // 加载配置
        │     ├─ {app_data_dir}/config.json 存在 → 解析
        │     ├─ 不存在 → 生成默认配置并写入文件
        │     ├─ 解析失败 → 使用默认值 + 警告日志
        │     └─ apply_env_overrides()                   // 环境变量覆盖
        │           ├─ AGENTPULSE_PORT → config.port
        │           ├─ AGENTPULSE_CHECK_INTERVAL → config.check_interval_secs
        │           ├─ AGENTPULSE_PYTHON → config.python
        │           └─ AGENTPULSE_POLL_INTERVAL → config.poll_interval_ms
        │
        ├─→ Database::new_in_memory()                    // 创建内存 SQLite 数据库
        │     └─→ init_schema()                          // 建表 (sessions, events)
        │
        ├─→ Arc<Mutex<Database>>                         // 共享数据库引用
        │
        ├─→ EventServer::start_shared(db, addr)          // 启动 HTTP 服务器（独立线程）
        │     ├─ addr = "127.0.0.1:{config.port}"       // 端口来自配置
        │     └─→ thread::spawn(run_server_loop)          // 在独立线程中处理请求
        │
        ├─→ process_checker::start(db, config.check_interval_secs)  // 启动进程检测（独立线程）
        │     └─→ thread::spawn(loop { sleep(Ns); check })
        │
        ├─→ resolve_python(config.python)                // 解析 Python 解释器
        │     ├─ config 指定 → 直接使用
        │     └─ config 未指定 → 探测 python3 → 回退 python
        │
        └─→ tauri::Builder::default()
              │
              ├─→ .plugin(tauri_plugin_opener)           // 注册 opener 插件（打开文件/目录）
              ├─→ .plugin(tauri_plugin_shell)            // 注册 shell 插件
              ├─→ .plugin(tauri_plugin_dialog)           // 注册 dialog 插件（消息框）
              │
              ├─→ .manage(AppState { db, config })       // 注入共享数据库 + 配置到 Tauri State
              │
              ├─→ .invoke_handler(commands)              // 注册 IPC 命令（9 个）
              │
              └─→ .setup(move |app| {                    // 应用就绪回调
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
                          └─→ ensure_hooks_installed(..., python)  // 向 settings.json 写入 6 个 hook
                    })
```

## 关键时序说明

1. **日志最先初始化** — 在其他模块之前，确保所有启动日志被捕获
2. **配置其次加载** — 日志就绪后立即加载，后续模块使用配置值
3. **数据库初始化** — 在启动 HTTP 服务器和进程检测器之前，确保数据层就绪
4. **HTTP 服务器在独立线程** — `thread::spawn`，不阻塞 Tauri 主线程，端口来自配置
5. **进程检测器在独立线程** — 启动后立即进入循环，间隔来自配置
6. **Hooks 自动安装异步执行** — 在 `setup` 闭包中的独立线程，不阻塞窗口创建，Python 解释器来自配置
7. **所有线程共享同一个 `Arc<Mutex<Database>>`** — 通过 Tauri State 和 clone 传递

## 模块初始化顺序

```
时间线 ─────────────────────────────────────────────────────→

1. tracing 日志初始化
2. config.json 加载 + 环境变量覆盖
3. Database 创建 + Schema 初始化
4. EventServer 线程启动 (端口来自配置，默认 17878)
5. ProcessChecker 线程启动 (间隔来自配置，默认 5s)
6. Python 解释器解析
7. Tauri Builder 开始
   ├─ 插件注册 (opener, shell, dialog)
   ├─ State 注入 (db + config)
   ├─ IPC 命令注册 (9 个)
   └─ setup 回调
       ├─ 托盘创建
       ├─ 窗口关闭拦截
       └─ Hooks 自动安装（异步线程，使用已解析的 Python）
8. 窗口显示
```

## 错误处理策略

- `tracing::init()` 失败 → 回退到 stderr 输出，不 panic
- `AgentPulseConfig::load()` 失败 → 使用默认配置，记录警告
- `Database::new_in_memory()` 失败 → `expect` panic（不可恢复，没有数据库应用无意义）
- HTTP 服务器绑定失败 → 错误日志 + 终止线程（应用继续运行但无法接收事件）
- Hooks 安装失败 → 仅记录错误日志，不阻止应用启动（用户可手动安装）
- Tray 创建失败 → `setup` 返回 `Err` 传播给 Tauri
