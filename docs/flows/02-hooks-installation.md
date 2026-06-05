# 2. Hooks 安装与配置流程

## 涉及文件

- [hooks.rs](../../apps/desktop/src-tauri/src/hooks.rs) — Rust 端 hook 管理（应用内自动安装）
- [install_hooks.py](../../adapters/claude-code/install_hooks.py) — Python CLI 工具（用户手动安装）
- [commands.rs](../../apps/desktop/src-tauri/src/commands.rs) — Tauri IPC 命令（hook 操作）

## 概述

AgentPulse 通过 Claude Code 的 hooks 机制接收事件。Hooks 配置写入 `~/.claude/settings.json`，指向 `monitor_hook.py` 脚本。

有 **两种安装方式**：
1. **自动安装** — 应用启动时在 `setup()` 中后台执行（推荐）
2. **手动安装** — 用户运行 `python install_hooks.py`（备用）

## 流程 A: 应用自动安装（启动时）

```
lib::run() → setup(|app|)
  │
  └─→ thread::spawn {
        ├─→ app_handle.path().resource_dir()           // 获取资源目录
        ├─→ app_handle.path().app_data_dir()           // 获取应用数据目录
        ├─→ resolve("~/.claude/settings.json")         // 获取 CC 配置文件路径
        │
        ├─→ hooks::extract_monitor_script(resource_dir, app_data_dir)
        │     │
        │     ├─→ find_monitor_script(resource_dir)    // 定位 monitor_hook.py
        │     │     ├─→ 尝试: resource_dir/monitor_hook.py  (bundled)
        │     │     └─→ 回退: 源码树 adapters/claude-code/monitor_hook.py (dev)
        │     │
        │     └─→ fs::copy(src, dst)                   // 复制到 app_data_dir
        │           (仅当源文件更新或目标不存在时)
        │
        └─→ hooks::ensure_hooks_installed(settings_path, monitor_path)
              │
              ├─→ load_settings(settings_path)         // 读取现有 settings.json
              ├─→ build_hook_configs(monitor_script)   // 构建 6 个 hook 配置
              ├─→ 对比现有 hooks 是否一致
              │     ├─→ 全部一致 → return "already_ok"
              │     └─→ 有差异 →
              │           ├─→ backup_settings(path)    // 备份 settings.json → .json.bak
              │           ├─→ merge: 保留非 AgentPulse hooks，覆盖/插入 6 个事件
              │           └─→ save_settings(path)      // 写入文件
              │                 ├─→ had_any → "updated"
              │                 └─→ none → "installed"
              └─→ log::info!(status)
      }
```

## 流程 B: 用户手动安装/卸载

### install_hooks.py 命令行接口

```
python install_hooks.py                # 安装 (幂等)
python install_hooks.py --remove       # 卸载
python install_hooks.py --status       # 查看状态
python install_hooks.py --dry-run      # 预览变更
python install_hooks.py --force        # 强制重装
```

### 安装流程

```
main()
  │
  ├─→ get_adapter_path()                              // monitor_hook.py 绝对路径
  ├─→ build_hook_configs(adapter_path)                // 构建 6 个 hook 配置
  ├─→ load_settings(DEFAULT_SETTINGS_PATH)            // 读取 ~/.claude/settings.json
  │
  ├─→ get_hook_status(settings)                       // 检查已安装的 hooks
  │     └─→ 全部已安装 & !force → "already_installed"
  │
  ├─→ backup: settings.json → settings.json.bak       // 备份
  ├─→ merge_hooks(settings, hook_configs)             // 合并 hooks
  └─→ save_settings(settings_path, new_settings)      // 写入
```

### 卸载流程

```
main() --remove
  │
  ├─→ settings_path 不存 → "no_settings_file"
  ├─→ backup: settings.json → settings.json.bak
  ├─→ remove_hooks_from_settings()                    // 移除 6 个 hook 事件
  └─→ save_settings()
```

### 状态查询

```
main() --status
  │
  ├─→ load_settings()
  ├─→ get_hook_status()
  └─→ 打印每个事件的状态:
        [OK] SessionStart
        [OK] PreToolUse
        [OK] PostToolUse
        [OK] PostToolUseFailure
        [OK] Notification
        [OK] Stop
```

## 6 个 Hook 事件的配置格式

写入 `~/.claude/settings.json` 的 `hooks` 字段：

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "python \"C:\\path\\to\\monitor_hook.py\"" }
        ]
      }
    ],
    "PreToolUse": [ /* ... */ ],
    "PostToolUse": [ /* ... */ ],
    "PostToolUseFailure": [ /* ... */ ],
    "Notification": [ /* ... */ ],
    "Stop": [ /* ... */ ]
  }
}
```

## 关键设计决策

1. **幂等性** — `ensure_hooks_installed()` 在每次启动时运行，但只在 hooks 缺失/过时时修改文件
2. **保留其他 hooks** — 只管理 6 个 AgentPulse 事件，不影响用户自定义 hooks
3. **自动备份** — 每次修改前备份 `settings.json` → `settings.json.bak`
4. **路径自动修复** — 当 `monitor_hook.py` 位置变化时（如应用更新），自动更新 hook 中的路径（状态 `"updated"`）
5. **Rust 与 Python 双重实现** — Rust 端用于应用内自动安装，Python 脚本用于 CLI 和开发调试

## 错误处理

- `resource_dir()`/`app_data_dir()` 失败 → 记录错误日志，跳过安装
- `resolve()` 失败 → 记录错误日志，跳过安装
- settings.json 解析失败 → 视为空文件，从头创建
- 写入失败 → 返回 `Err`，记录错误日志
