# ClockOnDesk 桌面时钟

一个极简的桌面悬浮时钟：始终置顶、半透明、可拖动，默认停靠屏幕右上角。
基于 Tauri v2 + React 19 + TypeScript，macOS 已适配，架构上可直接移植 Windows/Linux。

## 功能

- 大字号数字钟 `HH:MM` + 小号秒数，下方显示日期星期（如 `11月6日 周五`）
- 跟随系统浅色/深色外观实时切换
- 无边框、透明、圆角面板；不占 Dock、不进 Cmd+Tab（macOS Accessory 模式）
- 始终浮在所有窗口之上：`NSWindowLevel = NSStatusWindowLevel(25)` + `collectionBehavior = canJoinAllSpaces | fullScreenAuxiliary`，
  可覆盖普通窗口（Tauri 自带的 `set_visible_on_all_workspaces` 只设了 `canJoinAllSpaces`，不够）
- **全屏场景兜底**：macOS 不允许第三方 App 的窗口覆盖别的 App 全屏 space，此时用两招补位：
  1. 菜单栏托盘常驻显示 `HH:mm`（全屏时鼠标移到屏幕顶部即可看到）；
  2. 全局快捷键 `Cmd/Ctrl+Shift+T` 弹出系统通知显示当前时间（通知可浮在全屏 App 之上）；若悬浮窗被隐藏，此快捷键也会把它重新显示
- 按住即可拖动到任意位置，位置自动记忆；外接屏拔掉导致位置越界时自动回落右上角
- 右键菜单 / 菜单栏托盘图标共用一套菜单：透明度、字号、显示秒、显示日期、始终置顶、显示在所有桌面空间、重置位置、隐藏、退出
- 托盘左键单击：显示 / 隐藏时钟
- 设置持久化到 `~/Library/Application Support/com.xiaw.clockondesk/settings.json`

## 开发

```bash
pnpm install
pnpm tauri dev        # 开发调试（前端端口 1421，可在 vite.config.ts 修改）
```

## 构建

```bash
pnpm tauri build --bundles app   # 产物：src-tauri/target/release/bundle/macos/ClockOnDesk.app
pnpm tauri build                 # 完整打包（含 dmg）
```

## 图标

源图在 `assets/`（SVG）。修改后重新生成：

```bash
pnpm icons                 # 渲染 app-icon.png 与托盘图标
pnpm tauri icon app-icon.png   # 生成全平台图标集到 src-tauri/icons/
```

## 代码结构

```
src/                       前端（React）
  App.tsx                  面板容器：主题跟随系统、右键菜单、内容尺寸上报
  components/Clock.tsx     时间 / 秒 / 日期渲染
  hooks/useNow.ts          对齐整秒的心跳，避免 setInterval 漂移
  hooks/useSettings.ts     设置加载与 settings-changed 事件订阅
  hooks/useSystemTheme.ts  prefers-color-scheme 监听
  lib/tauri.ts             invoke/listen 封装（支持浏览器降级预览样式）
src-tauri/src/             Rust 侧
  lib.rs                   窗口事件（关闭即隐藏、移动即记忆）、菜单分发
  settings.rs              Settings 模型、JSON 持久化、patch 合并
  window.rs                右上角定位、越界回落、右锚定的窗口自适应（fit_window）
  menu.rs                  菜单构建、托盘、菜单事件处理
```

## 两个容易踩的坑

1. **窗口尺寸上报不能用 `requestAnimationFrame` 驱动**：窗口被遮挡或不在当前桌面空间时，macOS 会暂停 rAF，
   首次 `fit_window` 永远不会触发，窗口就停在配置里的初始尺寸。现在改为挂载时同步测量 + `setTimeout` 补测。
2. **坐标体系**：`Monitor::size()` 返回物理像素，而 `Monitor::position()` / NSWindow 原点是逻辑点。
   两者混用会让窗口跑到屏幕外。本项目的窗口几何全部用逻辑点计算，窗口尺寸直接用 CSS 像素（与逻辑点 1:1）。

## 移植其他平台注意事项

- 窗口配置（透明/无边框/置顶/跳过任务栏）在 `src-tauri/tauri.conf.json`，Windows 需 WebView2，Linux 需要桌面合成器支持透明
- `Accessory` 激活策略与 `all_workspaces` 为 macOS 专属，已用 `#[cfg(target_os = "macos")]` 隔离
- 透明度通过 CSS 背景 alpha 实现（不依赖各平台 `set_opacity` 的差异）
