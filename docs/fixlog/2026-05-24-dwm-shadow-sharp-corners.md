# BugFix: Windows 透明窗口四角出现半透明尖角框

**日期:** 2026-05-24
**严重程度:** 中（视觉缺陷，核心功能正常）
**状态:** 已修复

## 现象

AgentPulse 浮动面板设置为透明无边框窗口（`transparent: true` + `decorations: false`），CSS `border-radius: 12px` 使 WebView 内容呈现圆角。但窗口实际显示时，圆角外侧存在一层**半透明的尖角边框**，即 CSS 圆角套在原生窗口的直角阴影内，形成不协调的视觉效果。

## 排查过程

1. **审查 Tauri 窗口配置** — `transparent: true` + `decorations: false` 已正确设置
2. **审查 CSS** — `.floating-panel` 和 `html, body` 均已设置 `background: transparent`
3. **对比已知 Tauri 透明窗口方案** — Windows DWM 默认在窗口周围渲染阴影，对透明无边框窗口也不例外
4. **确认根因** — `shadow` 属性默认为 `true`，DWM 阴影在四角形成可见的半透明直角轮廓

## 根因分析

```
Windows DWM (Desktop Window Manager)
  └── 渲染窗口阴影 (shadow: true，默认)
        └── 阴影是直角的，与 CSS border-radius: 12px 不匹配
              └── 四角可见：CSS 圆角内容 + DWM 直角阴影 = 尖角框套圆角
```

CSS 只能控制 WebView 内部的渲染形状，无法影响原生窗口级别的阴影形状。必须禁止操作系统为窗口添加阴影。

## 修复

**2 处修改**（commit `7348d96`）：

1. **tauri.conf.json** — 添加 `"shadow": false`，禁用 DWM 窗口阴影
2. **main.css** — `html, body` 添加 `border-radius: 12px`，确保 WebView 根层也与面板圆角一致

修复原理：
- `shadow: false` → 操作系统不再渲染直角阴影，消除尖角框
- `border-radius: 12px` 在根层 → WebView 内容从根开始就是圆角，无直角内容穿透

## 验证

- Rust 编译 ✅
- 视觉效果：需 `npm run tauri dev` 确认四角干净圆角、无尖角框

## 相关文件

- [apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json) — `"shadow": false`
- [apps/desktop/src/assets/main.css](../../apps/desktop/src/assets/main.css) — `border-radius: 12px`
