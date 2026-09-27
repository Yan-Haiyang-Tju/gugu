# 往咕咕的数据库里灌一批演示任务，方便第一次启动就能看到真实排版效果。
# 只在数据库不存在时执行，不会覆盖你自己的数据。
import os
import sqlite3
import sys
from datetime import datetime

DB_DIR = os.path.join(os.environ["APPDATA"], "com.yanhaiyang.gugu")
DB = os.path.join(DB_DIR, "gugu.db")

SCHEMA = """
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
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
"""

# (截止时间, 标题, 分类, 重要, 已完成)
TASKS = [
    ("2026-09-21T10:00:00", "需求评审", "work", 0, 1),
    ("2026-09-22T19:00:00", "健身", "health", 0, 1),
    ("2026-09-23T10:00:00", "组会", "work", 0, 1),
    ("2026-09-23T15:00:00", "实验 · 跑数据", "study", 0, 1),
    ("2026-09-24T18:30:00", "部门聚餐", "social", 0, 1),
    ("2026-09-25T16:30:00", "交周报", "work", 0, 1),
    ("2026-09-26T20:00:00", "读书会", "social", 0, 1),
    ("2026-09-27T08:00:00", "晨跑 30 分钟", "health", 0, 1),
    ("2026-09-27T10:00:00", "导师组会 · 汇报进展", "work", 0, 1),
    ("2026-09-27T14:00:00", "写实验报告 第三章", "study", 0, 0),
    ("2026-09-27T16:30:00", "交周报", "work", 1, 0),
    ("2026-09-27T19:00:00", "代码评审", "work", 0, 0),
    ("2026-09-27T21:00:00", "给妈妈打电话", "life", 0, 0),
    ("2026-09-28T09:30:00", "项目周会", "work", 0, 0),
    ("2026-09-28T15:00:00", "跑一次完整实验", "study", 0, 0),
    ("2026-09-30T14:00:00", "牙医预约", "life", 0, 0),
    ("2026-10-02T18:00:00", "和老张吃饭", "social", 0, 0),
    ("2026-10-05T10:00:00", "交月度总结", "work", 1, 0),
]


def main():
    if os.path.exists(DB):
        print(f"数据库已存在，跳过灌数据：{DB}")
        return 0
    os.makedirs(DB_DIR, exist_ok=True)
    conn = sqlite3.connect(DB)
    conn.executescript(SCHEMA)
    now = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
    for due, title, cat, pri, done in TASKS:
        conn.execute(
            "INSERT INTO tasks (title, notes, due_at, category, priority, done, done_at,"
            " remind_offsets, fired_offsets, repeat_rule, created_at)"
            " VALUES (?, '', ?, ?, ?, ?, ?, '[10,0]', '[10,0]', NULL, ?)",
            (title, due, cat, pri, done, now if done else None, now),
        )
    conn.commit()
    print(f"已写入 {len(TASKS)} 条演示任务 → {DB}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
