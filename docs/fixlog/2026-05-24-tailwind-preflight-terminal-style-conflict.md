# BugFix: Tailwind v4 preflight 与终端紧凑风格冲突

**日期:** 2026-05-24
**严重程度:** 中（视觉效果不符合设计稿，但功能正常）
**状态:** 已修复

## 现象

Terminal Pulse 风格改造完成后，实际运行效果与视觉伴侣设计稿不一致：文字行距过松、组件间距偏大、整体缺乏终端的紧凑感。

## 排查过程

1. **对比设计稿与代码** — 视觉伴侣 mockup 使用裸 HTML inline style，无外部 CSS 干扰；实际项目使用 Tailwind v4
2. **审查 Vite 配置** — 确认 `@tailwindcss/vite` 插件自动处理 `main.css` 中的 `@import "tailwindcss"`
3. **识别冲突来源** — Tailwind v4 的 preflight/base 层注入以下样式：
   - `html { line-height: 1.5 }` — 1.5 倍行高远大于终端常用的 1.2
   - `*, ::before, ::after { box-sizing: border-box }` — 改变盒模型，padding 计入 width/height
4. **计算内容溢出** — 原 `padding: 14px` + `line-height: 1.5` 导致空状态实际高度超过 `minHeight: 64px`

## 根因分析

Tailwind v4 preflight 的 `line-height: 1.5` 是全局生效的。视觉伴侣 mockup 没有引入 Tailwind CSS，浏览器默认 `line-height: normal`（≈1.2）。这 ~0.3 的行高差 × 多层嵌套，导致实际渲染比设计稿"松"了约 25%。

此外，`box-sizing: border-box` 使 padding 占据内部空间，配合紧凑窗口高度时内容溢出。

## 修复

**文件**: 5 个文件（commit `e7a763e`）

1. **main.css** — `html, body` 添加 `line-height: 1.2` + `font-size: 12px` 全局基准
2. **FloatingPanel.vue** — padding 14→10px，header 间距减半（8→4px, 6→4px），添加 `line-height: 1.2`
3. **SessionCard.vue** — 添加 `line-height: 1.2`
4. **ExpandedDetail.vue** — 添加 `line-height: 1.25`
5. **tauri.conf.json** — height/minHeight 64→72 配合新间距

## 验证

- TypeScript 类型检查 ✅
- Rust 编译 ✅
- 视觉效果：需 `npm run tauri dev` 确认与视觉伴侣设计稿一致

## 相关文件

- [apps/desktop/src/assets/main.css](../../apps/desktop/src/assets/main.css)
- [apps/desktop/src/components/FloatingPanel.vue](../../apps/desktop/src/components/FloatingPanel.vue)
- [apps/desktop/src/components/SessionCard.vue](../../apps/desktop/src/components/SessionCard.vue)
- [apps/desktop/src/components/ExpandedDetail.vue](../../apps/desktop/src/components/ExpandedDetail.vue)
- [apps/desktop/src-tauri/tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json)

## 经验教训

- 使用 Tailwind v4 的项目中，`@import "tailwindcss"` 的 preflight 会修改全局排版属性，紧凑型终端/CLI 风格 UI 必须显式覆盖 `line-height`
- 视觉伴侣 mockup 使用裸 HTML 验证设计，但必须意识到实际项目中的 CSS 框架会引入额外样式
