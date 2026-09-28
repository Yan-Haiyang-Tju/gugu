//! Windows 平台特有逻辑：桌面钉扎、窗口样式、全局「双击 Ctrl」热键、全屏检测。
//!
//! 这里是整个项目唯一直接调用 Win32 的地方。之所以不用第三方热键库：
//! 连按两次 Ctrl 这种「无修饰键组合」的手势必须走 WH_KEYBOARD_LL 低级钩子，
//! 现成库都只支持标准组合键。

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;

use tauri::AppHandle;
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
    QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::*;

// ---------- Win32 常量 ----------
// 直接写字面量而不是引用 windows crate 的常量，避免不同大版本改名导致的编译失败。
const WM_KEYDOWN: usize = 0x0100;
const WM_KEYUP: usize = 0x0101;
const WM_SYSKEYDOWN: usize = 0x0104;
const WM_SYSKEYUP: usize = 0x0105;
const VK_CONTROL: u32 = 0x11;
const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const WH_KEYBOARD_LL: i32 = 13;
/// 让 Progman 分裂出承载壁纸的 WorkerW 窗口（Rainmeter 同款做法）
const WM_SPAWN_WORKER: u32 = 0x052C;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================
//  桌面钉扎：把窗口挂到 WorkerW 上，成为「壁纸层」的一部分
// ============================================================

/// 找到可以挂载窗口的「壁纸层」WorkerW。
///
/// Windows 11（本机 26200 实测）的桌面结构：
/// ```text
/// Progman
///  ├─ SHELLDLL_DefView   ← 桌面图标层
///  └─ WorkerW            ← 壁纸层，z-order 在图标层之后
/// ```
/// 挂到那个 WorkerW 上，窗口就正好落在图标**下方** —— 这正是桌面小部件要的效果：
/// 看得见、但既不挡图标也不挡其它窗口。
///
/// 老版本 Windows 上 Progman 默认没有这个子 WorkerW，需要先发 0x052C 让它分裂出来，
/// 所以这里保留了一条兼容分支。
fn find_worker_w() -> HWND {
    unsafe {
        // GetShellWindow 比 FindWindow("Progman") 更可靠，拿到的就是桌面窗口
        let progman = GetShellWindow();
        if progman.is_invalid() {
            return HWND::default();
        }

        // 1) 新版本 Windows：壁纸层已经作为 Progman 的子窗口存在
        if let Some(w) = worker_child_of(progman) {
            return w;
        }

        // 2) 老版本：发消息让 Progman 分裂出 WorkerW，再找一次
        let mut result: usize = 0;
        let _ = SendMessageTimeoutW(
            progman,
            WM_SPAWN_WORKER,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(&mut result),
        );
        if let Some(w) = worker_child_of(progman) {
            return w;
        }

        // 3) 更老的结构：图标宿主窗口之后的兄弟 WorkerW
        let defview = wide("SHELLDLL_DefView");
        if let Ok(view) = FindWindowExW(Some(progman), None, PCWSTR(defview.as_ptr()), PCWSTR::null())
        {
            if !view.is_invalid() {
                let workerw = wide("WorkerW");
                if let Ok(w) =
                    FindWindowExW(None, Some(progman), PCWSTR(workerw.as_ptr()), PCWSTR::null())
                {
                    if !w.is_invalid() {
                        return w;
                    }
                }
            }
        }

        HWND::default()
    }
}

// user32 的公开导出，声明链接即可（不必为此打开 Win32_System_Threading 整个模块）
#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(idattach: u32, idattachto: u32, fattach: i32) -> i32;
}

/// 解除跨进程 SetParent 造成的输入队列挂接。
///
/// 踩到的坑：SetParent 到 explorer 的窗口时，Windows 会把本线程的输入队列和
/// explorer 的挂在一起。后果是**桌面（Progman）为前台时，低级键盘钩子收不到按键**，
/// 表现就是「在别的窗口双击 Ctrl 好使，回到桌面就失灵」。
/// 挂接本身对「挂在图标下方纯展示」没有任何用处，所以挂完立刻解除。
unsafe fn detach_input_queue(window: HWND, target: HWND) {
    let ours = unsafe { GetWindowThreadProcessId(window, None) };
    let theirs = unsafe { GetWindowThreadProcessId(target, None) };
    if ours != 0 && theirs != 0 && ours != theirs {
        unsafe {
            // 两个方向都试一次：AttachThreadInput 是方向敏感的，
            // 只解一个方向可能解不干净。
            let _ = AttachThreadInput(ours, theirs, 0);
            let _ = AttachThreadInput(theirs, ours, 0);
        }
    }
}

fn worker_child_of(parent: HWND) -> Option<HWND> {
    let workerw = wide("WorkerW");
    unsafe {
        FindWindowExW(Some(parent), None, PCWSTR(workerw.as_ptr()), PCWSTR::null())
            .ok()
            .filter(|w| !w.is_invalid())
    }
}

/// 切换小部件所在的层级：
/// - `above_icons = false` → 挂在壁纸层 WorkerW 里，落在桌面图标**下方**（不挡桌面，但收不到点击）
/// - `above_icons = true`  → 直接挂到 Progman 下并置顶，浮到图标**上方**（可点击可拖动，会盖住图标）
///
/// 面板打开时切到后者，关闭时切回前者——这样既能随时操作，平时又不占地方。
pub fn set_widget_layer(hwnd: HWND, above_icons: bool) -> bool {
    unsafe {
        let desktop = GetShellWindow();
        if desktop.is_invalid() {
            return false;
        }
        let target = if above_icons {
            desktop
        } else {
            let w = find_worker_w();
            if w.is_invalid() {
                eprintln!("[gugu] 未找到桌面壁纸层（WorkerW），小部件将留在普通窗口层");
                return false;
            }
            w
        };

        // 换父窗口前后要按屏幕坐标重新摆一次（坐标系跟着父窗口变）
        let before = screen_rect(hwnd);

        // 跨进程 SetParent 前必须保证是 WS_CHILD，否则 Windows 静默忽略
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        style = (style & !WS_POPUP.0) | WS_CHILD.0;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        let _ = SetParent(hwnd, Some(target));
        let _ = SetWindowPos(
            hwnd,
            Some(HWND::default()), // HWND_TOP：在同级里置顶
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_FRAMECHANGED.0),
        );

        // 挂完顺手解除输入队列挂接（跨进程 SetParent 的已知副作用，解除无害）
        detach_input_queue(hwnd, target);

        let ok = GetParent(hwnd).unwrap_or_default() == target;
        // 挂进桌面层后坐标相对父窗口，必须显式换算一次
        if let Some((x, y, _, _)) = before {
            place_on_screen(hwnd, x, y);
        }
        println!(
            "[gugu] 层级切换 above_icons={above_icons} 目标={:?} 成功={ok} 窗口可见={}",
            target.0,
            IsWindowVisible(hwnd).as_bool()
        );
        ok
    }
}

// ---------- 非公开 API：给窗口加系统级模糊 ----------
// DWM 的 windowEffects（亚克力/云母）只对顶层窗口生效，小部件被挂成 explorer 的子窗口后
// 就失效了。SetWindowCompositionAttribute 是 Win10 时代就存在的非公开接口，
// 对子窗口依然有效，是这里唯一能拿到"真磨砂"的途径。

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    accent_flags: u32,
    /// ABGR：0xAABBGGRR
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttributeData {
    attribute: u32,
    data: *mut core::ffi::c_void,
    size_of_data: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_DISABLED: u32 = 0;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

type SetWcaFn = unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> i32;

/// 取 SetWindowCompositionAttribute 的地址并缓存。
/// 它虽然由 user32.dll 导出，却不在 user32.lib 的导入表里，直接链接会 LNK2019，
/// 只能运行时解析。取不到就返回 None，调用方安静降级成实心底色。
fn set_wca() -> Option<SetWcaFn> {
    static FN: OnceLock<Option<SetWcaFn>> = OnceLock::new();
    *FN.get_or_init(|| unsafe {
        const PROC: &[u8] = b"SetWindowCompositionAttribute\0";
        let dll = wide("user32.dll");
        let user32 = GetModuleHandleW(PCWSTR(dll.as_ptr())).ok()?;
        let addr = GetProcAddress(user32, PCSTR::from_raw(PROC.as_ptr()))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            SetWcaFn,
        >(addr))
    })
}

/// 给窗口加上（或去掉）系统级背景模糊。`dark` 决定薄雾的色调。
pub fn enable_backdrop_blur(hwnd: HWND, on: bool, dark: bool) {
    let Some(func) = set_wca() else {
        return;
    };
    // 浅色用偏白的薄雾，深色压暗；最高字节的 alpha 决定磨砂浓淡。
    // 取 0x99（6 成）是刻意的折中：即使这个非公开接口在某些机器上不生效，
    // CSS 那层底色仍能把内容撑到可读。
    let tint: u32 = if dark { 0x99_24_20_1E } else { 0x99_F3_F6_F7 };
    let mut policy = AccentPolicy {
        accent_state: if on { ACCENT_ENABLE_ACRYLICBLURBEHIND } else { ACCENT_DISABLED },
        accent_flags: 0,
        gradient_color: tint,
        animation_id: 0,
    };
    let mut data = WindowCompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: &mut policy as *mut _ as *mut core::ffi::c_void,
        size_of_data: std::mem::size_of::<AccentPolicy>(),
    };
    let ok = unsafe { func(hwnd, &mut data) } != 0;
    BLUR_OK.store(on && ok, Ordering::Relaxed);

    // 这个接口是未公开的，不同 Windows 版本表现不一，失败时打个日志便于定位
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !ok && !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("[gugu] 系统磨砂不可用，小部件回退为实心底色");
    }
}

static BLUR_OK: AtomicBool = AtomicBool::new(false);

/// 系统磨砂当前是否真的生效。
/// 前端据此决定小部件用半透明还是实心底色——拿不到磨砂还做半透明的话，
/// 桌面图标会清晰地透上来，正是要避免的情况。
pub fn backdrop_blur_available() -> bool {
    BLUR_OK.load(Ordering::Relaxed)
}

/// 设置窗口的扩展样式。
///
/// `activatable` 是操作模式的关键：挂成跨进程子窗口时窗口不能激活，
/// 而 WebView2 只在能激活的普通窗口里处理鼠标输入。
pub fn apply_widget_styles(hwnd: HWND, click_through: bool, activatable: bool) {
    unsafe {
        let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        ex |= WS_EX_TOOLWINDOW.0; // 不出现在 Alt+Tab
        if activatable {
            ex &= !WS_EX_NOACTIVATE.0;
        } else {
            ex |= WS_EX_NOACTIVATE.0;
        }
        if click_through {
            ex |= WS_EX_TRANSPARENT.0;
        } else {
            ex &= !WS_EX_TRANSPARENT.0;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    }
}

/// 把窗口从桌面层里摘出来，变成置顶的普通顶层窗口。
///
/// 这是「操作模式」：跨进程子窗口收不到鼠标（WebView2 的输入链路在那种状态下是断的，
/// 实测合成点击后任务状态不变化），所以要让用户能点能拖，就必须脱离桌面层。
pub fn detach_and_raise(hwnd: HWND) {
    unsafe {
        let Some((x, y, w, h)) = screen_rect(hwnd) else {
            return;
        };
        let _ = SetParent(hwnd, None);
        // SetParent(NULL) 不会自动把 WS_CHILD 换回 WS_POPUP
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        style = (style & !WS_CHILD.0) | WS_POPUP.0;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

        // 注意：子窗口转成顶层窗口时，同一个数值会被按新的坐标空间重新解释，
        // 所以这里必须把屏幕坐标显式写回去，不能图省事用 SWP_NOMOVE。
        // 用 HWND_TOP 而不是 HWND_TOPMOST：抬到「当前画面之上」就够了，
        // 仍留在普通窗口层里，用户打开别的应用时能正常把它盖住。
        if let Err(e) = SetWindowPos(
            hwnd,
            Some(HWND::default()), // 先只调位置，层级稍后统一处理
            x,
            y,
            w,
            h,
            SET_WINDOW_POS_FLAGS(SWP_FRAMECHANGED.0 | SWP_SHOWWINDOW.0),
        ) {
            eprintln!("[gugu] 摆放小部件失败：{e}");
        }
        force_to_front(hwnd);
    }
}

/// 抬到「当前画面之上」，但保持普通窗口身份。
///
/// 只调 `SetWindowPos(HWND_TOP)` 是不够的：从后台进程调用时它压不过当前前台窗口
/// （实测窗口抬完仍排在前台应用之下，用户看不到）。经典做法是先置顶、再立刻退回
/// 普通层——两次调用之后窗口落在普通窗口带的最前面，既盖住了当前画面，
/// 又没有变成永久置顶，用户打开别的应用时照样能正常盖住它。
pub fn force_to_front(hwnd: HWND) {
    const HWND_TOPMOST: isize = -1;
    const HWND_NOTOPMOST: isize = -2;
    unsafe {
        let flags = SET_WINDOW_POS_FLAGS(SWP_NOMOVE.0 | SWP_NOSIZE.0);
        let _ = SetWindowPos(hwnd, Some(HWND(HWND_TOPMOST as *mut _)), 0, 0, 0, 0, flags);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND(HWND_NOTOPMOST as *mut _)),
            0,
            0,
            0,
            0,
            flags,
        );
    }
}

/// 把普通窗口抬到同层的最前面（不改变置顶状态）
pub fn raise_window(hwnd: HWND) {
    force_to_front(hwnd);
}

/// 该窗口是不是当前的前台窗口（用来判断「已经在最前面」还是「被盖住了」）
pub fn is_foreground(hwnd: HWND) -> bool {
    unsafe { GetForegroundWindow() == hwnd }
}


/// 取窗口在屏幕坐标系中的位置（钉扎成子窗口后 GetWindowRect 仍返回屏幕坐标）
pub fn screen_rect(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r).ok()?;
        Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
    }
}

/// 按屏幕坐标摆放窗口。钉扎后子窗口坐标相对父窗口，需要减掉父窗口原点。
pub fn place_on_screen(hwnd: HWND, x: i32, y: i32) {
    unsafe {
        let (mut x, mut y) = (x, y);
        if let Ok(parent) = GetParent(hwnd) {
            if !parent.is_invalid() {
                let mut pr = RECT::default();
                if GetWindowRect(parent, &mut pr).is_ok() {
                    x -= pr.left;
                    y -= pr.top;
                }
            }
        }
        let _ = SetWindowPos(
            hwnd,
            None,
            x,
            y,
            0,
            0,
            SET_WINDOW_POS_FLAGS(SWP_NOSIZE.0 | SWP_NOZORDER.0),
        );
    }
}

// ============================================================
//  全屏检测：独占全屏 / 演示模式下隐藏小部件
// ============================================================

pub fn is_fullscreen_app() -> bool {
    unsafe {
        match SHQueryUserNotificationState() {
            Ok(state) => {
                let s = state.0;
                s == QUNS_BUSY.0
                    || s == QUNS_RUNNING_D3D_FULL_SCREEN.0
                    || s == QUNS_PRESENTATION_MODE.0
            }
            Err(_) => false,
        }
    }
}

// ============================================================
//  全局热键：400ms 内连按两次「独立的 Ctrl」
// ============================================================

static HOOK: AtomicIsize = AtomicIsize::new(0);
static ENABLED: AtomicBool = AtomicBool::new(true);
static WINDOW_MS: AtomicU64 = AtomicU64::new(420);
/// 上一次「独立 Ctrl」抬起的时间；0 表示暂无基准
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
/// 本次 Ctrl 按下期间有没有夹带其它按键（有则不算独立按下，避免 Ctrl+C 误触）
static ALONE: AtomicBool = AtomicBool::new(true);
static TRIGGER_TX: OnceLock<Sender<()>> = OnceLock::new();
/// 单调时钟基准（钩子回调里不能做重活，用最轻的方式取时间差）
static BASE_MS: OnceLock<std::time::Instant> = OnceLock::new();

fn now_ms() -> u64 {
    BASE_MS.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

/// 钩子通道自己的「上一次独立 Ctrl 抬起」时间（轮询通道另有一份本地状态）
static LAST_UP: AtomicU64 = AtomicU64::new(0);
/// 最近一次「非 Ctrl 按键」的时间。轮询通道靠它判断这记 Ctrl 是不是独立按下的，
/// 避免 Ctrl+C / Ctrl+V 连按被误判成双击 Ctrl。
static LAST_OTHER_KEY: AtomicU64 = AtomicU64::new(0);
/// 触发冷却：只用来吸收两条通道之间几十毫秒的重复触发。
///
/// **不能设大**。设成 600ms 时，用户「唤出后立刻再按一次收起来」这种正常操作
/// 会被整个吞掉，表现就是「偶尔按了没反应」。两条通道对同一次手势的触发
/// 相隔只有几十毫秒，250ms 足够去重。
static LAST_FIRE: AtomicU64 = AtomicU64::new(0);
const FIRE_COOLDOWN_MS: u64 = 250;

/// 两条通道共用的触发入口。`src` 只用于日志，便于确认是哪条通道生效。
fn fire_trigger(src: &str) {
    let now = now_ms();
    if now.saturating_sub(LAST_FIRE.load(Ordering::Relaxed)) < FIRE_COOLDOWN_MS {
        return; // 同一次手势，另一条通道已经处理过了
    }
    LAST_FIRE.store(now, Ordering::Relaxed);
    println!("[gugu] 热键触发（{src}）");
    if let Some(tx) = TRIGGER_TX.get() {
        let _ = tx.send(());
    }
}

unsafe extern "system" fn kb_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // code < 0 时必须原样透传；code == 0 即 HC_ACTION
    if code == 0 && ENABLED.load(Ordering::Relaxed) {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // 钩子回调里绝不能做 I/O：打印会拖慢回调，超过系统的 ~300ms 上限后
        // 钩子会被静默摘除。诊断时踩过这个坑——加了日志反而丢事件。
        let injected = kb.flags.0 & LLKHF_INJECTED.0 != 0;
        // 发布版忽略合成按键，避免被其它程序的模拟输入误触；
        // 调试版放行，这样自动化脚本才能验证整条热键链路。
        let ignore = injected && !cfg!(debug_assertions);
        if !ignore {
            let vk = kb.vkCode;
            let is_ctrl = vk == VK_CONTROL || vk == VK_LCONTROL || vk == VK_RCONTROL;
            let msg = wparam.0;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                if is_ctrl {
                    CTRL_DOWN.store(true, Ordering::Relaxed);
                    ALONE.store(true, Ordering::Relaxed);
                } else if vk != 0 {
                    // vk == 0 是无效按键：某些键盘驱动、输入法、以及带合成输入的
                    // 程序都会发出这种噪声事件。把它当成「用户按了别的键」会让
                    // 轮询通道在随后一段时间内拒绝工作，表现就是「偶尔按了没反应」。
                    LAST_OTHER_KEY.store(now_ms(), Ordering::Relaxed);
                    if CTRL_DOWN.load(Ordering::Relaxed) {
                        ALONE.store(false, Ordering::Relaxed);
                    }
                }
            } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                if is_ctrl {
                    let alone = ALONE.swap(false, Ordering::Relaxed);
                    CTRL_DOWN.store(false, Ordering::Relaxed);
                    if alone {
                        // 每条通道各自记录「上一次独立 Ctrl 抬起」。
                        // 不要共享：两条通道采样率不同（钩子是即时的、轮询 25ms 一次），
                        // 共享会让它们互相打乱节奏，反而漏掉或重复触发。
                        let now = now_ms();
                        let prev = LAST_UP.load(Ordering::Relaxed);
                        if prev != 0
                            && now.saturating_sub(prev) <= WINDOW_MS.load(Ordering::Relaxed)
                        {
                            LAST_UP.store(0, Ordering::Relaxed);
                            fire_trigger("钩子");
                        } else {
                            LAST_UP.store(now, Ordering::Relaxed);
                        }
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vkey: i32) -> i16;
}

/// Ctrl 状态轮询通道 —— 双击 Ctrl 的主通道。
///
/// 为什么不能只靠键盘钩子：钩子在某些前台状态下会被系统跳过。实测「桌面（Progman）
/// 为前台」时收不到按键，而其它窗口都正常，表现就是「在别的窗口能用、回桌面就失灵」。
/// 轮询不依赖钩子，也不会被系统摘除，代价只是每 25ms 一次极轻量的系统调用。
///
/// 防误触：只有「最近 800ms 内没按过其它键」时才认这记 Ctrl 是独立按下的，
/// 这样 Ctrl+C / Ctrl+V 这类连按不会把面板误唤出来。
fn spawn_ctrl_poller() {
    std::thread::spawn(|| {
        let mut was_down = false;
        let mut last_up: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(25));
            if !ENABLED.load(Ordering::Relaxed) {
                continue;
            }
            let down = unsafe {
                GetAsyncKeyState(VK_LCONTROL as i32) < 0
                    || GetAsyncKeyState(VK_RCONTROL as i32) < 0
            };
            if was_down && !down {
                let now = now_ms();
                // 500ms 足够把 Ctrl+C 这类组合排除掉（其它键紧随 Ctrl 之后按下），
                // 又不至于让一次无关按键把热键"封口"太久。
                //
                // 注意 last_other == 0 要单独放行：0 表示「本次运行还没按过其它键」，
                // 若直接参与相减会得到「现在距开机多少毫秒」，应用刚启动的那几百毫秒内
                // 会恒小于 500ms，把轮询通道自己锁死。
                let last_other = LAST_OTHER_KEY.load(Ordering::Relaxed);
                let quiet = last_other == 0 || now.saturating_sub(last_other) > 500;
                let win = WINDOW_MS.load(Ordering::Relaxed);
                if !quiet {
                    // 这记 Ctrl 夹带了别的键（Ctrl+C 之类），不算独立按下
                    last_up = 0;
                } else if last_up != 0 && now.saturating_sub(last_up) <= win {
                    last_up = 0;
                    fire_trigger("轮询");
                } else {
                    last_up = now;
                }
            }
            was_down = down;
        }
    });
}

/// 安装低级键盘钩子，并起一个消费触发的常驻线程。
///
/// 钩子装在**独立线程**上（不是主线程），这点很关键：低级钩子的回调由安装它的
/// 线程的消息循环驱动，回调超时（系统默认 300ms）会被 Windows 静默摘除，热键从此失效。
/// 主线程要跑窗口操作，其中跨进程 SetParent 可能阻塞上百毫秒甚至更久，
/// 装在主线程上就会被连带摘掉——表现正是「第一次能用，之后就失灵」。
pub fn install_hotkey(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = TRIGGER_TX.set(tx);

    // 消费者：钩子回调里只做一次 send，真正的活交给主线程事件循环
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            let handle = app.clone();
            let inner = handle.clone();
            if handle
                .run_on_main_thread(move || crate::toggle_panel(&inner))
                .is_err()
            {
                eprintln!("[gugu] 热键回调投递失败（主线程事件循环不可用）");
            }
        }
    });

    spawn_ctrl_poller();

    // 临时验证轮询通道：跳过钩子安装
    if std::env::var("GUGU_SKIP_HOOK").is_ok() {
        println!("[gugu] 调试：已跳过键盘钩子，仅使用轮询通道");
        return;
    }

    // 钩子线程：装好钩子后必须一直跑消息循环，否则回调不会被调用
    std::thread::spawn(|| unsafe {
        let hmod = GetModuleHandleW(PCWSTR::null())
            .ok()
            .map(|m| HINSTANCE(m.0));
        match SetWindowsHookExW(WINDOWS_HOOK_ID(WH_KEYBOARD_LL), Some(kb_proc), hmod, 0) {
            Ok(h) => {
                HOOK.store(h.0 as isize, Ordering::SeqCst);
                println!("[gugu] 双击 Ctrl 热键已就绪");
            }
            Err(e) => {
                eprintln!("[gugu] 热键钩子安装失败：{e}");
                return;
            }
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

pub fn set_hotkey_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn set_hotkey_window(ms: u64) {
    WINDOW_MS.store(ms.clamp(200, 900), Ordering::Relaxed);
}
