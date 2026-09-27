//! 提醒引擎：常驻后台线程，每 30 秒扫一次截止时间。
//!
//! 放在 Rust 侧而不是前端定时器，是因为小部件窗口长期被其它窗口遮挡，
//! WebView2 会把被遮挡窗口的 JS 定时器降频甚至挂起，提醒会不准时。

use std::time::Duration;

use chrono::NaiveDateTime;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::Store;
use crate::TIME_FMT;

/// 已过期多久之后不再补发提醒（避免关机一晚后开机被一堆通知淹没）
const GRACE_MINUTES: i64 = 5;

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        loop {
            tick(&app);
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}

fn tick(app: &AppHandle) {
    let store = app.state::<Store>();
    let settings = store.settings();
    let dnd = settings.get("dnd").and_then(|v| v.as_bool()).unwrap_or(false);

    if !dnd {
        let now = crate::now_naive();
        if let Ok(tasks) = store.due_tasks() {
            for task in tasks {
                let Ok(due) = NaiveDateTime::parse_from_str(&task.due_at, TIME_FMT) else {
                    continue;
                };
                let mut fired = task.fired.clone();
                let mut changed = false;
                for offset in &task.remind_offsets {
                    if fired.contains(offset) {
                        continue;
                    }
                    let trigger = due - chrono::Duration::minutes(*offset);
                    if now >= trigger && now < trigger + chrono::Duration::minutes(GRACE_MINUTES) {
                        notify(app, &task.title, &task.due_at, *offset);
                        fired.push(*offset);
                        changed = true;
                    } else if now >= trigger + chrono::Duration::minutes(GRACE_MINUTES) {
                        // 早就过了时点：静默标记为已提醒，不再打扰
                        fired.push(*offset);
                        changed = true;
                    }
                }
                if changed {
                    let _ = store.mark_fired(task.id, &fired);
                }
            }
        }
    }

    crate::apply_widget_visibility(app);
    crate::tray::refresh_tooltip(app);
}

fn notify(app: &AppHandle, title: &str, due_at: &str, offset: i64) {
    let when = due_at.get(11..16).unwrap_or("");
    let body = if offset <= 0 {
        format!("{when} 到点了 · {title}")
    } else {
        format!("{offset} 分钟后（{when}）· {title}")
    };
    if let Err(e) = app
        .notification()
        .builder()
        .title("咕咕 · 任务提醒")
        .body(body)
        .show()
    {
        eprintln!("[gugu] 发送通知失败：{e}");
    }
}
