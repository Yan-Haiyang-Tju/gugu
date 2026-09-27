//! 托盘图标：悬停显示下一个任务倒计时，右键菜单控制常用开关。

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::db::Store;
use tauri_plugin_autostart::ManagerExt;

pub const TRAY_ID: &str = "gugu";

/// 读一个布尔设置
fn flag(app: &AppHandle, key: &str, default: bool) -> bool {
    app.state::<Store>()
        .settings()
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "打开面板", true, Some("Ctrl Ctrl"))?;
    let add = MenuItem::with_id(app, "add", "快速添加任务…", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let dnd = CheckMenuItem::with_id(
        app,
        "dnd",
        "免打扰",
        true,
        flag(app, "dnd", false),
        None::<&str>,
    )?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启",
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let widget = CheckMenuItem::with_id(
        app,
        "widget",
        "显示桌面小部件",
        true,
        flag(app, "widgetVisible", true),
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出咕咕", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &open, &add, &sep1, &dnd, &widget, &autostart, &sep2, &settings, &quit,
        ],
    )
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("咕咕 · 桌面日历")
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::toggle_panel(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    let store = app.state::<Store>();
    match id {
        "open" => crate::show_panel(app),
        "add" => crate::show_panel_with(app, "panel:add"),
        "settings" => crate::show_panel_with(app, "panel:settings"),
        "dnd" => {
            let next = !flag(app, "dnd", false);
            store.update_settings(&serde_json::json!({ "dnd": next }));
            let _ = app.emit("settings:changed", store.settings());
            refresh_menu(app);
        }
        "widget" => {
            let next = !flag(app, "widgetVisible", true);
            store.update_settings(&serde_json::json!({ "widgetVisible": next }));
            crate::apply_widget_visibility(app);
            refresh_menu(app);
        }
        "autostart" => {
            let next = !app.autolaunch().is_enabled().unwrap_or(false);
            let result = if next {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };
            if let Err(e) = result {
                eprintln!("[gugu] 设置开机自启失败：{e}");
            }
            store.update_settings(&serde_json::json!({ "autostart": next }));
            refresh_menu(app);
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

/// 重建菜单以反映最新的勾选状态（比持有 CheckMenuItem 句柄更省心）
pub fn refresh_menu(app: &AppHandle) {
    if let Ok(menu) = build_menu(app) {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// 托盘悬停提示：优先显示最近的一个待办
pub fn refresh_tooltip(app: &AppHandle) {
    let store = app.state::<Store>();
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let tip = match store.next_task() {
        Ok(Some(task)) => {
            let when = task
                .due_at
                .as_deref()
                .and_then(|s| s.get(11..16))
                .unwrap_or("--:--");
            match &task.due_at {
                Some(due) if crate::is_overdue(due) => {
                    format!("咕咕 · 逾期：{} {}", when, task.title)
                }
                _ => format!("咕咕 · 下一个 {} {}", when, task.title),
            }
        }
        _ => "咕咕 · 今天没有待办了".to_string(),
    };
    let _ = tray.set_tooltip(Some(&tip));
}
