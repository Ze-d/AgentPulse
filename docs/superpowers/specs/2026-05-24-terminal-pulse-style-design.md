# Design: Terminal Pulse 风格改造

**日期:** 2026-05-24
**状态:** 已确认，待实现

## 概述

将 AgentPulse 浮动面板从通用暗色 UI 改造为**终端/CLI 风格**的工具伴侣，与 Claude Code 的命令行体验形成视觉呼应。

## 设计目标

1. **排版终生化** — 全等宽字体，CLI prompt 风格标题，彩色文字状态
2. **布局自适应** — 面板高度根据 session 数量自动伸缩，空状态最小化
3. **修复白边** — 窗口透明化，border-radius 圆角外真正透明

---

## 一、排版：终端风格

### 字体栈

```
'Cascadia Code', 'JetBrains Mono', 'Fira Code', 'Consolas', monospace
```

优先级：Cascadia Code（Windows Terminal 默认）> JetBrains Mono（IDE 常见）> Consolas（Windows 系统自带）> monospace（fallback）

### 字号层级

| 元素 | 字号 | 字重 | 说明 |
|------|------|------|------|
| 标题 prompt | 13px | 700 | `~/agentpulse $` CLI 风格 |
| 活跃计数 | 11px | 400 | `[N active]` 方括号格式 |
| 卡片项目名 | 12px | 600 | `cc > project-name` 路径风格 |
| 卡片状态 | 11px | 400 | 彩色文字：running/waiting/failed |
| 卡片时长 | 11px | 400 | 右对齐 |
| 卡片工具名 | 11px | 400 | 灰色，右对齐 |
| 空状态文字 | 11px | 400 | `$ waiting for hooks...` |

### 标题格式

```
~/agentpulse $                        [2 active]
─────────────────────────────────────────────────  (border-bottom)
```

- prompt 符号 `$` 用 mauve 色
- 计数用方括号 `[N active]` 而非纯数字

### 状态表示

**移除**彩色圆点 (`.status-dot`)，**改用**彩色文字：

| 状态 | 颜色 | 显示 |
|------|------|------|
| running | green `#a6e3a1` | `running` |
| tool_running | blue `#89b4fa` | `tool` |
| waiting_input | yellow `#f9e2af` | `waiting` |
| waiting_permission | red `#f38ba8` | `permission` |
| completed | overlay0 `#6c7086` | `done` |
| failed | red `#f38ba8` | `failed` |

### 卡片布局（终端行式）

```
cc > project-name                    1m  running
                                        Bash
```

- 第一行：来源 + 项目名（左）| 时长 + 状态（右）
- 第二行：工具名（右对齐，仅当有值时显示）

---

## 二、布局：自适应高度

### 方案

面板宽度固定 320px，高度使用 Tauri Window API 动态调整：

**空状态:** 窗口高度 ≈ 60-70px（仅标题 + 一行提示）
**有 session:** 根据卡片数量自动计算，最大受限于 `maxHeight: 420`
**展开详情:** 详情区域展开需要额外高度

### 高度计算逻辑

```
header:     40px (标题行 + border)
card:       36px × N sessions
padding:    28px (14px × 2)
max height: 420px
min height: 64px
```

### Tauri 窗口调整

在 `sessionStore` 中每次 `fetchSessions` 后调用窗口 resize：

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";

async function adjustWindowSize(cardCount: number, expanded: boolean) {
  const headerHeight = 40;
  const cardHeight = 36;
  const padding = 28;
  const expandedExtra = expanded ? 120 : 0;

  const contentHeight = cardCount > 0
    ? headerHeight + cardCount * cardHeight + padding + expandedExtra
    : 64; // empty state

  const height = Math.min(Math.max(contentHeight, 64), 420);

  await getCurrentWindow().setSize(new LogicalSize(320, height));
}
```

### 配置变更

`tauri.conf.json` 窗口配置调整：
```json
{
  "height": 64,
  "minHeight": 64,
  "maxHeight": 420
}
```

---

## 三、白边修复：透明窗口

### 问题

CSS `border-radius: 12px` 裁剪 WebView 内容，但原生 Tauri 窗口是直角矩形。圆角外区域显示窗口默认背景色（白色），形成四角白边。

### 修复

1. **tauri.conf.json** — 添加 `"transparent": true`
2. **main.css** — `html, body { background: transparent; }`
3. **App.vue** — 保持 `background: transparent`
4. **FloatingPanel.vue** — `.floating-panel` 保持 `background: var(--color-base)` + `border-radius: 12px`

这样圆角外的区域透过透明窗口看到真正的桌面背景。

---

## 四、涉及文件

| 文件 | 变更 |
|------|------|
| `apps/desktop/src-tauri/tauri.conf.json` | `transparent: true`，`height: 64`，`minHeight: 64`，`maxHeight: 420` |
| `apps/desktop/src/assets/main.css` | font-family 全局设置 |
| `apps/desktop/src/App.vue` | background: transparent |
| `apps/desktop/src/components/FloatingPanel.vue` | 终端风格 header，卡片布局，空状态，自适应高度逻辑 |
| `apps/desktop/src/components/SessionCard.vue` | 重构为终端行式布局，移除状态圆点 |
| `apps/desktop/src/components/ExpandedDetail.vue` | 终端风格展开详情 |
| `apps/desktop/src/types/agent.ts` | 可能需要更新 STATUS_LABELS 为短标签 |
| `apps/desktop/src/stores/sessionStore.ts` | 添加 adjustWindowSize 逻辑 |
| `apps/desktop/src-tauri/capabilities/default.json` | 添加 `core:window:allow-set-size` 权限 |

## 五、兼容性说明

- `transparent: true` 在 Windows 10/11 上均支持
- 等宽字体 fallback 链确保所有平台可渲染
- 自适应高度依赖 Tauri v2 Window API，已经是项目依赖

## 六、不复用的现有样式

- `.status-dot` — 移除，改用彩色文字
- SessionCard 中的 flex 多行布局 — 改为终端行式
- `.empty-state` 的居中大段文字 — 改为单行 CLI 风格
- `panel-header` 的 `justify-content: space-between` 标题+计数 — 保留但改为 prompt 格式
- `.needs-attention` 脉冲动画 — 考虑替换为终端 bell 风格闪烁
