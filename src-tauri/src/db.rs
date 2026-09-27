//! SQLite 存储层：任务与设置的持久化。
//!
//! 时间统一用本地时间字符串 `YYYY-MM-DDTHH:MM:SS` 存储 —— 可读、可排序，
//! 且避免了跨时区转换带来的歧义（这是一个单机桌面日历，不存在跨时区场景）。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

/// 一条任务。通过 serde 以 camelCase 暴露给前端。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub notes: String,
    /// 截止时间；`None` 表示无截止（不参与时间轴与提醒）
    pub due_at: Option<String>,
    /// work | study | life | health | social | errand
    pub category: String,
    /// 0 普通 / 1 重要
    pub priority: i64,
    pub done: bool,
    pub done_at: Option<String>,
    /// 提前多少分钟提醒，0 表示准点；空数组表示不提醒
    pub remind_offsets: Vec<i64>,
    pub repeat_rule: Option<String>,
    pub created_at: String,
}

impl Task {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let raw: String = row.get("remind_offsets")?;
        Ok(Task {
            id: row.get("id")?,
            title: row.get("title")?,
            notes: row.get("notes")?,
            due_at: row.get("due_at")?,
            category: row.get("category")?,
            priority: row.get("priority")?,
            done: row.get::<_, i64>("done")? != 0,
            done_at: row.get("done_at")?,
            remind_offsets: serde_json::from_str(&raw).unwrap_or_default(),
            repeat_rule: row.get("repeat_rule")?,
            created_at: row.get("created_at")?,
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// 前端提交的新建 / 编辑表单
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    /// 有 id 为编辑，无 id 为新建
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub remind_offsets: Option<Vec<i64>>,
    #[serde(default)]
    pub repeat_rule: Option<String>,
}

/// 提醒扫描需要的精简投影（含已触发记录，不暴露给前端）
pub struct DueTask {
    pub id: i64,
    pub title: String,
    pub due_at: String,
    pub remind_offsets: Vec<i64>,
    pub fired: Vec<i64>,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        // WAL 让「提醒线程读」与「界面写」不互相阻塞
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        self.lock().execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                title          TEXT    NOT NULL,
                notes          TEXT    NOT NULL DEFAULT '',
                due_at         TEXT,
                category       TEXT    NOT NULL DEFAULT 'errand',
                priority       INTEGER NOT NULL DEFAULT 0,
                done           INTEGER NOT NULL DEFAULT 0,
                done_at        TEXT,
                remind_offsets TEXT    NOT NULL DEFAULT '[10,0]',
                fired_offsets  TEXT    NOT NULL DEFAULT '[]',
                repeat_rule    TEXT,
                created_at     TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(done, due_at);
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // 锁只会在持有期间 panic 时中毒；此处直接恢复，避免整个应用卡死
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn list_tasks(&self) -> rusqlite::Result<Vec<Task>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT * FROM tasks ORDER BY (due_at IS NULL), due_at ASC, done ASC, id ASC",
        )?;
        let rows = stmt.query_map([], Task::from_row)?;
        rows.collect()
    }

    pub fn get_task(&self, id: i64) -> rusqlite::Result<Option<Task>> {
        let conn = self.lock();
        conn.query_row("SELECT * FROM tasks WHERE id = ?1", params![id], Task::from_row)
            .optional()
    }

    /// 新建或更新任务。返回落库后的完整记录。
    pub fn save_task(&self, input: &TaskInput) -> rusqlite::Result<Task> {
        let title = input.title.trim().to_string();
        let notes = input.notes.clone().unwrap_or_default();
        let category = input
            .category
            .clone()
            .unwrap_or_else(|| "errand".to_string());
        let priority = input.priority.unwrap_or(0);
        let remind = input.remind_offsets.clone().unwrap_or_else(|| vec![10, 0]);
        let remind_json = serde_json::to_string(&remind).unwrap_or_else(|_| "[10,0]".into());
        let now = crate::now_string();

        let id = match input.id {
            Some(id) => {
                // 截止时间或提醒设置变了，就重新武装提醒（否则保留已触发记录）
                let old = self.get_task(id)?;
                let rearm = match &old {
                    Some(o) => o.due_at != input.due_at || o.remind_offsets != remind,
                    None => true,
                };
                let conn = self.lock();
                if rearm {
                    conn.execute(
                        "UPDATE tasks SET title=?2, notes=?3, due_at=?4, category=?5, priority=?6,
                         remind_offsets=?7, fired_offsets='[]', repeat_rule=?8 WHERE id=?1",
                        params![
                            id, title, notes, input.due_at, category, priority,
                            remind_json, input.repeat_rule
                        ],
                    )?;
                } else {
                    conn.execute(
                        "UPDATE tasks SET title=?2, notes=?3, due_at=?4, category=?5, priority=?6,
                         remind_offsets=?7, repeat_rule=?8 WHERE id=?1",
                        params![
                            id, title, notes, input.due_at, category, priority,
                            remind_json, input.repeat_rule
                        ],
                    )?;
                }
                id
            }
            None => {
                let conn = self.lock();
                conn.execute(
                    "INSERT INTO tasks (title, notes, due_at, category, priority,
                                        remind_offsets, fired_offsets, repeat_rule, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, '[]', ?7, ?8)",
                    params![
                        title, notes, input.due_at, category, priority,
                        remind_json, input.repeat_rule, now
                    ],
                )?;
                conn.last_insert_rowid()
            }
        };

        self.get_task(id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn set_done(&self, id: i64, done: bool) -> rusqlite::Result<Option<Task>> {
        let done_at = if done { Some(crate::now_string()) } else { None };
        {
            let conn = self.lock();
            // 取消完成时重新武装提醒，避免"完成后又取消"导致提醒不再触发
            conn.execute(
                "UPDATE tasks SET done=?2, done_at=?3 WHERE id=?1",
                params![id, done as i64, done_at],
            )?;
        }
        self.get_task(id)
    }

    pub fn delete_task(&self, id: i64) -> rusqlite::Result<()> {
        self.lock()
            .execute("DELETE FROM tasks WHERE id=?1", params![id])?;
        Ok(())
    }

    /// 未完成、且带截止时间的任务：提醒线程使用
    pub fn due_tasks(&self) -> rusqlite::Result<Vec<DueTask>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, due_at, remind_offsets, fired_offsets
             FROM tasks WHERE done = 0 AND due_at IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            let remind: String = row.get("remind_offsets")?;
            let fired: String = row.get("fired_offsets")?;
            Ok(DueTask {
                id: row.get("id")?,
                title: row.get("title")?,
                due_at: row.get("due_at")?,
                remind_offsets: serde_json::from_str(&remind).unwrap_or_default(),
                fired: serde_json::from_str(&fired).unwrap_or_default(),
            })
        })?;
        rows.collect()
    }

    pub fn mark_fired(&self, id: i64, fired: &[i64]) -> rusqlite::Result<()> {
        let json = serde_json::to_string(fired).unwrap_or_else(|_| "[]".into());
        self.lock().execute(
            "UPDATE tasks SET fired_offsets=?2 WHERE id=?1",
            params![id, json],
        )?;
        Ok(())
    }

    /// 下一个待办（托盘提示与快捷键提示用）
    pub fn next_task(&self) -> rusqlite::Result<Option<Task>> {
        let conn = self.lock();
        conn.query_row(
            "SELECT * FROM tasks WHERE done = 0 AND due_at IS NOT NULL
             ORDER BY due_at ASC LIMIT 1",
            [],
            Task::from_row,
        )
        .optional()
    }

    // ---------- 设置 ----------

    pub fn settings(&self) -> serde_json::Value {
        let stored: Option<String> = {
            let conn = self.lock();
            conn.query_row(
                "SELECT value FROM settings WHERE key='app'",
                [],
                |r| r.get(0),
            )
            .optional()
            .unwrap_or(None)
        };
        let mut base = crate::default_settings();
        if let Some(raw) = stored {
            if let (Some(base_map), Ok(saved)) = (
                base.as_object_mut(),
                serde_json::from_str::<serde_json::Value>(&raw),
            ) {
                if let Some(saved_map) = saved.as_object() {
                    for (k, v) in saved_map {
                        base_map.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        base
    }

    /// 合并式更新：只覆盖传入的键，返回合并后的完整设置
    pub fn update_settings(&self, patch: &serde_json::Value) -> serde_json::Value {
        let merged = {
            let mut current = self.settings();
            if let (Some(cur), Some(p)) = (current.as_object_mut(), patch.as_object()) {
                for (k, v) in p {
                    cur.insert(k.clone(), v.clone());
                }
            }
            current
        };
        let raw = serde_json::to_string(&merged).unwrap_or_else(|_| "{}".into());
        let _ = self.lock().execute(
            "INSERT INTO settings (key, value) VALUES ('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![raw],
        );
        merged
    }
}
