//! 咕咕（GUGU）· 常驻桌面的日历任务小部件
//!
//! 两个窗口共用一套前端产物，按 `?window=widget|panel` 分流：
//! - `widget` 钉在桌面壁纸层，常驻显示月历与今日任务，不抢焦点
//! - `panel` 平时隐藏，双击 Ctrl 从屏幕中央唤出，用于查看与编辑

mod db;
mod remind;
mod tray;
mod win;

use db::{Store, Task, TaskInput};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub const TIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

// ============================================================
//  时间与默认设置
// ============================================================

pub fn now_string() -> String {
    chrono::Local::now().naive_local().format(TIME_FMT).to_string()
}

pub fn now_naive() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

pub fn is_overdue(due_at: &str) -> bool {
    chrono::NaiveDateTime::parse_from_str(due_at, TIME_FMT)
        .map(|d| d < now_naive())
        .unwrap_or(false)
}

pub fn default_settings() -> serde_json::Value {
    json!({
        "theme": "celadon",       // celadon 青瓷绿 | azure 天青蓝 | tangerine 蜜柑橘
        "mode": "light",          // light | dark
        "glassAlpha": 0.82,       // 玻璃面板透明度 0.50–0.95
        "blur": true,             // 毛玻璃
        "widgetVisible": true,
        // 平时小部件待在桌面哪一层（两种都是纯展示，交互一律走快捷键升到顶层）：
        // float = 浮在桌面图标之上（更显眼，但会盖住那块图标）
        // wallpaper = 沉到图标之下（完全不占地方）
        "widgetLayer": "wallpaper",
        "clickThrough": false,    // 鼠标穿透（穿透后只能用热键唤出面板）
        "hotkeyEnabled": true,
        "hotkeyWindowMs": 420,    // 双击 Ctrl 两次之间的间隔上限
        "dnd": false,             // 免打扰
        "defaultRemindOffsets": [10, 0],
        "widgetX": -1,            // -1 表示尚未定位，按默认位置摆放
        "widgetY": -1,
        "widgetW": 300,
        "widgetH": 560,
        "autostart": false
    })
}

// ============================================================
//  窗口控制
// ============================================================

fn panel(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("panel")
}

fn widget(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("widget")
}

/// 操作模式：把小部件从桌面层摘出来置顶，让它可点可拖。
///
/// 平时小部件是 explorer 的子窗口（跨进程），那种状态下 WebView2 收不到鼠标输入，
/// 所以「能操作」和「在桌面层」二者只能取其一——默认沉在桌面，按快捷键才升起来。
fn set_operate_mode(app: &AppHandle, on: bool) {
    let t0 = std::time::Instant::now();
    let Some(w) = widget(app) else { return };
    let Ok(hwnd) = w.hwnd() else { return };
    let settings = app.state::<Store>().settings();
    let click_through = settings
        .get("clickThrough")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if on {
        win::detach_and_raise(hwnd);
    } else {
        win::set_widget_layer(hwnd, widget_layer_is_float(&settings));
    }
    win::apply_widget_styles(hwnd, click_through, on);

    // 这段跑在主线程上，里面的跨进程 SetParent 有可能阻塞；
    // 记一下耗时，超过百毫秒就说明它正是把键盘钩子拖超时（并导致其被摘除）的元凶。
    let cost = t0.elapsed().as_millis();
    if cost > 50 {
        println!("[gugu] 切换窗口层级耗时 {cost}ms（跨进程 SetParent 阻塞）");
    }

    // 让小部件知道自己当前能不能被操作，好在界面上给出提示
    let _ = app.emit("widget:mode", if on { "operate" } else { "idle" });
}

fn widget_layer_is_float(settings: &serde_json::Value) -> bool {
    settings
        .get("widgetLayer")
        .and_then(|v| v.as_str())
        .unwrap_or("wallpaper")
        == "float"
}

fn panel_is_open(app: &AppHandle) -> bool {
    panel(app)
        .map(|p| p.is_visible().unwrap_or(false))
        .unwrap_or(false)
}

/// 唤出面板并聚焦，同时通知前端切到对应视图
pub fn show_panel_with(app: &AppHandle, event: &str) {
    set_operate_mode(app, true);
    if let Some(w) = panel(app) {
        let _ = w.show();
        // 操作模式下小部件是置顶的，面板要是普通层级就会被它压住
        // （用户把小部件拖到面板上就会点不动面板）。把面板也置顶并后抬，保证它在上。
        let _ = w.set_always_on_top(true);
        let _ = w.set_focus();
        let _ = app.emit(event, ());
    }
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(w) = panel(app) {
        let _ = w.hide();
    }
    set_operate_mode(app, false);
}

pub fn show_panel(app: &AppHandle) {
    show_panel_with(app, "panel:show");
}

pub fn toggle_panel(app: &AppHandle) {
    let Some(w) = panel(app) else {
        eprintln!("[gugu] 切换面板失败：找不到 panel 窗口");
        return;
    };
    let visible = w.is_visible().unwrap_or(false);
    println!("[gugu] 切换面板：当前 visible={visible}");
    if visible {
        hide_panel(app);
    } else {
        show_panel(app);
    }
}

/// 按设置与全屏状态决定小部件是否露面
pub fn apply_widget_visibility(app: &AppHandle) {
    let Some(w) = widget(app) else { return };
    let want = app
        .state::<Store>()
        .settings()
        .get("widgetVisible")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    // 独占全屏 / 演示模式下让位，避免和游戏画面打架
    let should_show = want && !win::is_fullscreen_app();
    let visible = w.is_visible().unwrap_or(true);
    if should_show && !visible {
        let _ = w.show();
    } else if !should_show && visible {
        let _ = w.hide();
    }
}

/// 把设置里与窗口相关的部分落到窗口上
pub fn apply_settings(app: &AppHandle) {
    let settings = app.state::<Store>().settings();

    win::set_hotkey_enabled(
        settings
            .get("hotkeyEnabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    );
    win::set_hotkey_window(
        settings
            .get("hotkeyWindowMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(420),
    );

    if let Some(w) = widget(app) {
        if let Ok(hwnd) = w.hwnd() {
            let click_through = settings
                .get("clickThrough")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let blur = settings.get("blur").and_then(|v| v.as_bool()).unwrap_or(true);
            let dark = settings.get("mode").and_then(|v| v.as_str()) == Some("dark");
            let operating = panel_is_open(app);

            // 操作模式下窗口是顶层窗口，别把它按回桌面层
            // （widgetLayer = "none" 是排查用的档位：完全不挂桌面层，用来定位
            //   「点不动」到底是桌面层导致的还是别的原因）
            let layer_now = settings
                .get("widgetLayer")
                .and_then(|v| v.as_str())
                .unwrap_or("wallpaper");
            if !operating && layer_now != "none" {
                win::set_widget_layer(hwnd, widget_layer_is_float(&settings));
            }
            // DWM 的亚克力对子窗口无效，只有挂在桌面层时（非操作模式）才需要自己加磨砂
            win::enable_backdrop_blur(hwnd, blur && !operating, dark);
            win::apply_widget_styles(hwnd, click_through, operating);
        }
    }

    apply_widget_visibility(app);
    tray::refresh_menu(app);
}

/// 启动时决定小部件位置：优先用记忆的位置，落到屏幕外则回到右上角
fn restore_widget_position(app: &AppHandle) {
    let Some(w) = widget(app) else { return };
    let settings = app.state::<Store>().settings();
    let x = settings.get("widgetX").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
    let y = settings.get("widgetY").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
    let cw = settings.get("widgetW").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
    let ch = settings.get("widgetH").and_then(|v| v.as_i64()).unwrap_or(560) as i32;

    let on_screen = x >= 0
        && y >= 0
        && app.available_monitors().unwrap_or_default().iter().any(|m| {
            let mp = m.position();
            let ms = m.size();
            // 左上角一小块落在某块屏幕内即视为有效，避免拔掉副屏后窗口消失
            x + 48 > mp.x && x < mp.x + ms.width as i32 && y >= mp.y - 4 && y < mp.y + ms.height as i32
        });

    let (tx, ty) = if on_screen {
        (x, y)
    } else {
        default_widget_position(app, cw, ch)
    };
    let _ = w.set_position(PhysicalPosition::new(tx, ty));
}

fn default_widget_position(app: &AppHandle, cw: i32, _ch: i32) -> (i32, i32) {
    match app.primary_monitor() {
        Ok(Some(m)) => {
            let mp = m.position();
            let ms = m.size();
            (mp.x + ms.width as i32 - cw - 28, mp.y + 64)
        }
        _ => (1200, 80),
    }
}

// ============================================================
//  命令：任务
// ============================================================

#[tauri::command]
fn list_tasks(store: tauri::State<'_, Store>) -> Result<Vec<Task>, String> {
    store.list_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_task(
    app: AppHandle,
    store: tauri::State<'_, Store>,
    task: TaskInput,
) -> Result<Task, String> {
    let saved = store.save_task(&task).map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", ());
    Ok(saved)
}

#[tauri::command]
fn set_done(
    app: AppHandle,
    store: tauri::State<'_, Store>,
    id: i64,
    done: bool,
) -> Result<(), String> {
    store.set_done(id, done).map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", ());
    Ok(())
}

#[tauri::command]
fn delete_task(app: AppHandle, store: tauri::State<'_, Store>, id: i64) -> Result<(), String> {
    store.delete_task(id).map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", ());
    Ok(())
}

// ============================================================
//  命令：设置与窗口
// ============================================================

#[tauri::command]
fn get_settings(store: tauri::State<'_, Store>) -> serde_json::Value {
    store.settings()
}

/// 系统级磨砂当前是否可用（不可用时前端会把小部件画成实心卡片）
#[tauri::command]
fn blur_active() -> bool {
    win::backdrop_blur_available()
}

/// 开发期用：把前端的一条消息打进应用日志。
/// 用来确认「某个交互事件到底有没有触发」，比隔着窗口猜要快得多。
#[tauri::command]
fn debug_log(msg: String) {
    println!("[gugu:ui] {msg}");
}

/// 小部件当前是否处于操作模式。
///
/// 前端除了监听 widget:mode 事件，挂载时也要主动问一次：
/// 页面重载（开发期 HMR、或任何原因的重刷）会把事件期间设的标志丢掉，
/// 导致「窗口明明已经升起来了，界面却以为不能拖」。
#[tauri::command]
fn widget_operating(app: AppHandle) -> bool {
    panel_is_open(&app)
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    store: tauri::State<'_, Store>,
    patch: serde_json::Value,
) -> serde_json::Value {
    let merged = store.update_settings(&patch);
    apply_settings(&app);
    let _ = app.emit("settings:changed", merged.clone());
    merged
}

#[tauri::command]
fn show_panel_cmd(app: AppHandle) {
    show_panel(&app);
}

/// 唤出面板并直接进入新建任务（小部件的「＋」按钮）
#[tauri::command]
fn show_panel_add(app: AppHandle) {
    show_panel_with(&app, "panel:add");
}

/// 唤出面板并切到设置页
#[tauri::command]
fn show_panel_settings(app: AppHandle) {
    show_panel_with(&app, "panel:settings");
}

#[tauri::command]
fn hide_panel_cmd(app: AppHandle) {
    hide_panel(&app);
}

#[tauri::command]
fn toggle_panel_cmd(app: AppHandle) {
    toggle_panel(&app);
}

/// 从小部件点开某个任务：唤出面板并让它直接打开该任务的编辑窗
#[tauri::command]
fn open_task(app: AppHandle, id: i64) {
    if let Some(w) = panel(&app) {
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app.emit("panel:edit", id);
}

#[tauri::command]
fn focus_widget(app: AppHandle) {
    if let Some(w) = widget(&app) {
        let _ = w.set_focus();
    }
}

/// 记住小部件的位置与尺寸（前端在窗口移动后调用）
#[tauri::command]
fn save_widget_pos(app: AppHandle, store: tauri::State<'_, Store>) {
    let Some(w) = widget(&app) else { return };
    let Ok(hwnd) = w.hwnd() else { return };
    if let Some((x, y, cw, ch)) = win::screen_rect(hwnd) {
        store.update_settings(&json!({
            "widgetX": x, "widgetY": y, "widgetW": cw, "widgetH": ch
        }));
    }
}


#[tauri::command]
fn reset_widget_pos(app: AppHandle, store: tauri::State<'_, Store>) {
    let settings = store.settings();
    let cw = settings.get("widgetW").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
    let ch = settings.get("widgetH").and_then(|v| v.as_i64()).unwrap_or(560) as i32;
    let (x, y) = default_widget_position(&app, cw, ch);
    if let Some(w) = widget(&app) {
        let _ = w.set_position(PhysicalPosition::new(x, y));
        if let Ok(hwnd) = w.hwnd() {
            win::place_on_screen(hwnd, x, y);
        }
    }
    store.update_settings(&json!({ "widgetX": x, "widgetY": y }));
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, store: tauri::State<'_, Store>, on: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    if on {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    store.update_settings(&json!({ "autostart": on }));
    Ok(on)
}

#[tauri::command]
fn test_notification(app: AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title("咕咕 · 通知测试")
        .body("看到这条说明提醒功能是通的 ✓")
        .show();
}

/// 导出全部任务为 JSON 备份，返回文件路径
#[tauri::command]
fn export_data(app: AppHandle, store: tauri::State<'_, Store>) -> Result<String, String> {
    let tasks = store.list_tasks().map_err(|e| e.to_string())?;
    let payload = json!({
        "app": "gugu",
        "version": env!("CARGO_PKG_VERSION"),
        "exportedAt": now_string(),
        "tasks": tasks.iter().map(|t| t.to_json()).collect::<Vec<_>>(),
    });
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("gugu-{stamp}.json"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// 数据目录（数据库与备份都在这里）
#[tauri::command]
fn data_dir(app: AppHandle) -> String {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

// ============================================================
//  启动
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        // 单实例必须第一个注册：重复启动时唤起已有实例的面板
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_panel(app);
        }));
    }

    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let dir = app.path().app_data_dir()?;
            let store = Store::open(&dir.join("gugu.db"))?;
            app.manage(store);

            restore_widget_position(&handle);
            apply_settings(&handle);

            tray::build(&handle)?;
            win::install_hotkey(handle.clone());
            remind::spawn(handle.clone());

            // 面板不做「失焦自动收起」。
            //
            // 试过这个方案，但它很脆：Windows 有前台锁，set_focus 不一定真能抢到焦点，
            // 面板可能刚显示就判定自己失焦、立刻收起——连带把刚升起来的小部件也按回桌面层，
            // 表现就是「点了没反应」。改成完全由用户控制：
            // 开 = 双击 Ctrl，关 = Esc 或再按一次双击 Ctrl，状态清晰可预期。
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks,
            save_task,
            set_done,
            delete_task,
            get_settings,
            blur_active,
            debug_log,
            widget_operating,
            set_settings,
            show_panel_cmd,
            show_panel_add,
            show_panel_settings,
            hide_panel_cmd,
            toggle_panel_cmd,
            open_task,
            focus_widget,
            save_widget_pos,
            reset_widget_pos,
            get_autostart,
            set_autostart,
            test_notification,
            export_data,
            open_path,
            data_dir,
            quit_app
        ])
        .build(tauri::generate_context!())
        .expect("咕咕启动失败")
        .run(|_app, event| {
            // 关掉面板只是隐藏，程序继续驻留托盘
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
