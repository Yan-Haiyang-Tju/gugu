<div align="center">

# 咕咕 · GUGU

**一个住在壁纸上的日历 —— 平时不挡事，要用时一个快捷键唤出来。**

[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white)](#环境要求)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)

[English](README.md) · **简体中文**

<img src="docs/screenshots/widget.png" width="290" alt="咕咕桌面小部件">

</div>

---

## 为什么做这个

市面上的日历要么是个必须一直开着的重窗口，要么是个你从来不会去看的浏览器标签页。咕咕反着来：它待在**桌面壁纸层、桌面图标的下方**——你看向桌面时它一直在，你干活时它一次都不会挡到你。

真要用它的时候，连按两次 <kbd>Ctrl</kbd>：小部件升到你正在看的内容之上，编辑面板在旁边展开；用完了按 <kbd>Esc</kbd>，一切落回原位。

没有账号、没有同步、没有埋点。你的数据就是自己硬盘上的一个 SQLite 文件。

## 功能

**桌面小部件**
- 钉在壁纸层、**桌面图标下方** —— 看得见，但不占地方
- 日期可点选、复选框可直接勾、位置可拖动
- 三种范围：**今天** / **本周**（按天分组）/ **本月**
- 可选「浮在图标上」模式，想让它更显眼也行

**唤出面板** —— 连按两次 <kbd>Ctrl</kbd>
- 今天：纵向时间轴 + 实时「现在」线，逾期任务标红置顶
- 周：7 列时间网格
- 月：整月网格（带分类圆点）+ 当天列表
- 只升到**当前画面上方**，不会变成永久置顶窗口 —— 你切去别的应用，它照样会被盖住

**自然语言快速添加**
- 输入 `明天15:00 交周报 #工作 !重要`，日期 / 时间 / 标签 / 优先级实时解析成胶囊回显，确认后才建
- 认这些写法：`今天 明天 后天 周五 下周三 9月30日`、`15:00 / 15点 / 下午3点 / 晚上8点`、`#标签`、`!重要`、`每天 / 每周一`

**提醒**
- 走 Windows 原生 Toast 通知，由 Rust 侧的常驻线程触发
- 每个任务可单独设提前量（10 分钟 / 30 分钟 / 1 小时 / 1 天 / 准点 / 不提醒）
- 免打扰开关；检测到全屏应用时自动静默

**外观**
- 三套配色：青瓷绿 / 天青蓝 / 蜜柑橘
- 深浅色、透明度可调、毛玻璃可开关
- 托盘图标悬停显示下一个任务和倒计时

## 界面

| 小部件 · 今天 | 小部件 · 本周 |
| :---: | :---: |
| <img src="docs/screenshots/widget.png" width="255"> | <img src="docs/screenshots/widget-week.png" width="255"> |

| 面板 · 今天时间轴 | 面板 · 周网格 |
| :---: | :---: |
| <img src="docs/screenshots/panel-today.png" width="330"> | <img src="docs/screenshots/panel-week.png" width="330"> |

<div align="center">
<img src="docs/screenshots/panel-editor.png" width="330" alt="新建任务小窗">
</div>

## 用法

| 操作 | 方式 |
| --- | --- |
| 唤出 / 收起面板 | 快速连按两次 <kbd>Ctrl</kbd>（间隔约 420 ms 内）|
| 收起 | <kbd>Esc</kbd> |
| 完成任务 | 点复选框 —— 在桌面小部件上直接点就行 |
| 编辑任务 | 点任务行 |
| 移动小部件 | 拖标题栏（唤起后可拖）|
| 切换范围 | 小部件上的 `今天` / `本周` / `本月`，面板里的 `今天` / `周` / `月` |
| 新建任务 | 在快速添加框输入后回车，或点 `＋ 新建` |
| 设置 | 托盘图标 → 设置…，或面板里的 ⚙ |

热键间隔、小部件平时待在哪一层、透明度、配色、开机自启、数据导出，都在设置页里。

## 实现要点

两个窗口共用同一套前端产物，靠 `?window=widget|panel` 分流。有意思的部分都在 [`src-tauri/src/win.rs`](src-tauri/src/win.rs)。

**小部件住在桌面自己的窗口层里。** Windows 11 的桌面结构是：

```text
Progman
 ├─ SHELLDLL_DefView   ← 桌面图标
 └─ WorkerW            ← 壁纸层
```

把窗口挂到那个 `WorkerW` 上，它就落在图标下方。两个不显然的前提：

- 流传很广的「给 `Progman` 发 `0x052C` 让它分裂出 `WorkerW`」在当前 Windows 11 上**已经不需要**了 —— 壁纸层 `WorkerW` 本来就是 `Progman` 的子窗口。代码里保留了旧版本的兼容分支。
- 跨进程 `SetParent` 必须**先把子窗口的样式从 `WS_POPUP` 改成 `WS_CHILD`**，否则 Windows 会静默忽略：它返回成功，但什么都没做。判断是否挂上要用 `GetParent`，不能看返回值。

**「升到最前」不等于「永久置顶」。** 从后台进程调用 `SetWindowPos(hwnd, HWND_TOP, …)` 是压不过当前前台窗口的 —— 窗口只会排到它下面，用户什么也看不见。咕咕用的是经典两步：先 `HWND_TOPMOST`，紧接着 `HWND_NOTOPMOST`。这样窗口落在**普通窗口带的最前面**：盖住你此刻正在看的内容，但别的应用之后照样能盖住它。

**热键有两条独立通道。** 只靠低级键盘钩子（`WH_KEYBOARD_LL`）并不可靠：回调超过约 300 ms 的钩子会被 Windows 静默摘除，而且某些前台状态下按键根本不会送到钩子。所以在钩子之外，另有一个线程每 25 ms 轮询一次 `GetAsyncKeyState` —— 这两个问题它都不怕。两条通道共用一个带冷却的触发入口去重；「独立按下」的判定（800 ms 内没按过其它键）则保证 <kbd>Ctrl</kbd>+<kbd>C</kbd> 这类组合不会误触发。

**提醒跑在 Rust 里，不在 WebView 里。** 小部件长期被别的窗口遮挡，而 WebView2 会降频被遮挡窗口的定时器 —— 用 JS 做提醒会越走越偏甚至停掉。所以由 Rust 的常驻线程每 30 秒扫一次截止时间。

**毛玻璃需要兜底。** DWM 的亚克力/云母效果只对顶层窗口生效，小部件一旦成为桌面的子窗口就完全失效。咕咕会尝试非公开接口 `SetWindowCompositionAttribute`（运行时解析 —— 它由 `user32.dll` 导出，但不在 `user32.lib` 的导入表里），并把结果回报给前端。失败时小部件退回接近不透明的实心卡片，而不是维持半透明、导致文字看不清。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 外壳 | Tauri 2（Rust）—— 安装包约 5 MB，常驻内存约 40 MB |
| 前端 | Vue 3 + TypeScript + Vite，CSS 全部手写（不用组件库）|
| 存储 | SQLite（`rusqlite`，WAL 模式）|
| Windows 集成 | `windows` crate + 少量裸 FFI |

## 环境要求

- Windows 10 / 11（x64）。渲染依赖 WebView2 运行时，Windows 11 已自带。
- 从源码构建还需：[Rust](https://rustup.rs)、[Node.js 20+](https://nodejs.org)，以及 MSVC 构建工具（`Microsoft.VisualStudio.2022.BuildTools`，勾选「使用 C++ 的桌面开发」工作负载）。

## 开发

```bash
npm install
npm run tauri dev      # 起 Vite + 应用，支持热更新
npm run tauri build    # 产物是 NSIS 安装包，位于 src-tauri/target/release/bundle
```

只做前端的类型检查与构建用 `npm run build`。想给空数据库灌一批演示任务：

```bash
python tools/seed-demo.py
```

`tools/` 里还有开发期用到的只读诊断脚本（窗口树、z-order、命中测试、按 DPI 抓图）。请先读 [`tools/README.md`](tools/README.md) —— **运行任何会注入输入的脚本之前尤其要看**；另外 `tools/release-stuck-keys.ps1` 用于处理「脚本被打断导致修饰键卡住」的情况。

### 关于内存

在 16 GB 内存的机器上编译依赖 `windows` crate 的依赖树可能把内存吃光，并抛出极具误导性的错误（`only metadata stub found for rlib dependency core`、`STATUS_STACK_BUFFER_OVERRUN`）。遇到这些先限制 Cargo 的并发 —— 几乎从来不是工具链坏了：

```toml
# ~/.cargo/config.toml
[build]
jobs = 2
```

## 后续计划

- [ ] 循环任务 —— 表结构里已经存了 `RRULE` 字符串，展开逻辑还没做
- [ ] 周视图里拖拽改期
- [ ] 跨天 / 全天任务
- [ ] 数据导入（JSON 导出已可用）
- [ ] macOS / Linux —— 平台相关代码已收在 `win.rs` 里，接口面很小

## 参与贡献

欢迎提 Issue 和 PR。几点能让 review 快很多：

- Windows 相关代码请留在 `src-tauri/src/win.rs` 里。
- 排查小部件状态请用 `tools/` 里的脚本去看真实情况，别靠猜 —— Windows 的窗口层级有大量「返回成功但什么也没做」的行为。
- 提 PR 前跑一遍 `npm run build` 和 `cargo build`。

## 许可证

[MIT](LICENSE)
