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

        let ok = GetParent(hwnd).unwrap_or_default() == target;
        // 挂进桌面层后坐标相对父窗口，必须显式换算一次
        if let Some((x, y, _, _)) = before {
            place_on_screen(hwnd, x, y);
        }
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
        // -1 = HWND_TOPMOST
        let topmost = HWND(-1isize as *mut core::ffi::c_void);
        let _ = SetWindowPos(
            hwnd,
            Some(topmost),
            x,
            y,
            w,
            h,
            SET_WINDOW_POS_FLAGS(SWP_FRAMECHANGED.0 | SWP_SHOWWINDOW.0),
        );
    }
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
static LAST_UP: AtomicU64 = AtomicU64::new(0);
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
/// 本次 Ctrl 按下期间有没有夹带其它按键（有则不算独立按下，避免 Ctrl+C 误触）
static ALONE: AtomicBool = AtomicBool::new(true);
static TRIGGER_TX: OnceLock<Sender<()>> = OnceLock::new();
/// 单调时钟基准（钩子回调里不能做重活，用最轻的方式取时间差）
static BASE_MS: OnceLock<std::time::Instant> = OnceLock::new();

fn now_ms() -> u64 {
    BASE_MS.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

unsafe extern "system" fn kb_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // code < 0 时必须原样透传；code == 0 即 HC_ACTION
    if code == 0 && ENABLED.load(Ordering::Relaxed) {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let injected = kb.flags.0 & LLKHF_INJECTED.0 != 0;
        if !injected {
            let vk = kb.vkCode;
            let is_ctrl = vk == VK_CONTROL || vk == VK_LCONTROL || vk == VK_RCONTROL;
            let msg = wparam.0;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                if is_ctrl {
                    CTRL_DOWN.store(true, Ordering::Relaxed);
                    ALONE.store(true, Ordering::Relaxed);
                } else if CTRL_DOWN.load(Ordering::Relaxed) {
                    ALONE.store(false, Ordering::Relaxed);
                }
            } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                if is_ctrl {
                    let alone = ALONE.swap(false, Ordering::Relaxed);
                    CTRL_DOWN.store(false, Ordering::Relaxed);
                    if alone {
                        let now = now_ms();
                        let prev = LAST_UP.load(Ordering::Relaxed);
                        let win = WINDOW_MS.load(Ordering::Relaxed);
                        if prev != 0 && now.saturating_sub(prev) <= win {
                            LAST_UP.store(0, Ordering::Relaxed); // 消费掉这一对，三连击不会重复触发
                            if let Some(tx) = TRIGGER_TX.get() {
                                let _ = tx.send(());
                            }
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

/// 在主线程安装低级键盘钩子，并起一个消费触发的常驻线程。
pub fn install_hotkey(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = TRIGGER_TX.set(tx);
    std::thread::spawn(move || {
        // recv 阻塞等待，钩子回调里只做一次 send，绝不阻塞系统输入
        while rx.recv().is_ok() {
            let handle = app.clone();
            let inner = handle.clone();
            let _ = handle.run_on_main_thread(move || crate::toggle_panel(&inner));
        }
    });

    unsafe {
        // GetModuleHandleW 返回 HMODULE，而钩子要的是 HINSTANCE，两者底层同为模块句柄
        let hmod = GetModuleHandleW(PCWSTR::null())
            .ok()
            .map(|m| HINSTANCE(m.0));
        match SetWindowsHookExW(WINDOWS_HOOK_ID(WH_KEYBOARD_LL), Some(kb_proc), hmod, 0) {
            Ok(h) => {
                HOOK.store(h.0 as isize, Ordering::SeqCst);
                println!("[gugu] 双击 Ctrl 热键已就绪");
            }
            Err(e) => eprintln!("[gugu] 热键钩子安装失败：{e}"),
        }
    }
}

pub fn set_hotkey_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn set_hotkey_window(ms: u64) {
    WINDOW_MS.store(ms.clamp(200, 900), Ordering::Relaxed);
}
