// 全局状态。两个窗口各自持有一份，通过 Rust 侧广播的事件保持同步。

import { listen } from "@tauri-apps/api/event";
import { reactive } from "vue";
import { api, type Settings, type Task, type TaskInput } from "./api";
import { addDays, dayKey, fromStamp, startOfDay, toStamp } from "./time";

export const state = reactive({
  tasks: [] as Task[],
  settings: null as Settings | null,
  ready: false,
  /** 供时间轴与逾期判断使用的「现在」，定期刷新 */
  now: new Date(),
});

/**
 * 系统磨砂是否真的生效。小部件被钉成 explorer 的子窗口后，
 * 只有非公开接口 SetWindowCompositionAttribute 能给它加模糊，而它在部分机器上会失败。
 * 失败时绝不能用半透明底色——那样桌面图标会清晰地透上来，字都看不清。
 */
let systemBlur = false;

export function applyTheme(s: Settings) {
  const root = document.documentElement;
  root.dataset.theme = s.theme;
  root.dataset.mode = s.mode;
  root.dataset.blur = s.blur && systemBlur ? "on" : "off";
  root.style.setProperty("--glass-alpha", String(s.glassAlpha));
  root.classList.toggle("no-blur", !s.blur);
}

export async function refreshTasks() {
  state.tasks = await api.listTasks();
}

export async function initStore() {
  const [tasks, settings] = await Promise.all([api.listTasks(), api.getSettings()]);
  state.tasks = tasks;
  state.settings = settings;
  try {
    systemBlur = await api.blurActive();
  } catch {
    systemBlur = false;
  }
  applyTheme(settings);
  state.ready = true;

  // Rust 侧改动后广播，两个窗口据此刷新
  void listen("tasks:changed", () => void refreshTasks());
  void listen("settings:changed", (e) => {
    state.settings = e.payload as Settings;
    applyTheme(state.settings);
  });

  window.setInterval(() => {
    state.now = new Date();
  }, 20_000);
}

// ---------- 写操作 ----------

export async function saveTask(input: TaskInput): Promise<Task> {
  const saved = await api.saveTask(input);
  await refreshTasks();
  return saved;
}

export async function toggleDone(task: Task, done?: boolean) {
  const next = done ?? !task.done;
  task.done = next; // 先改本地，勾选立刻有反馈；随后的广播会带回权威数据
  task.doneAt = next ? toStamp(new Date()) : null;
  await api.setDone(task.id, next);
}

export async function removeTask(id: number) {
  await api.deleteTask(id);
  await refreshTasks();
}

/** 把任务顺延一天（界面上的「推到明天」） */
export async function postpone(task: Task) {
  const due = fromStamp(task.dueAt);
  if (!due) return;
  const next = addDays(due, 1);
  await api.saveTask({
    id: task.id,
    title: task.title,
    notes: task.notes,
    dueAt: toStamp(next),
    category: task.category,
    priority: task.priority,
    remindOffsets: task.remindOffsets,
    repeatRule: task.repeatRule,
  });
  await refreshTasks();
}

export async function patchSettings(patch: Partial<Settings>) {
  state.settings = await api.setSettings(patch);
  // 开关毛玻璃会让后端重新尝试加系统磨砂，结果可能变，所以要重新问一次
  if ("blur" in patch) {
    try {
      systemBlur = await api.blurActive();
    } catch {
      /* 保持原值 */
    }
  }
  applyTheme(state.settings);
}

// ---------- 查询 ----------

const byDue = (a: Task, b: Task) =>
  (fromStamp(a.dueAt)?.getTime() ?? 0) - (fromStamp(b.dueAt)?.getTime() ?? 0);

export function isOverdue(task: Task, now = state.now): boolean {
  if (task.done || !task.dueAt) return false;
  const due = fromStamp(task.dueAt);
  return !!due && due.getTime() < now.getTime();
}

/** 某天的全部任务，按截止时间排序 */
export function tasksOnDay(d: Date): Task[] {
  const key = dayKey(d);
  return state.tasks.filter((t) => t.dueAt && dayKey(fromStamp(t.dueAt)!) === key).sort(byDue);
}

/** 今天视图要展示的三组：逾期 / 待办 / 已完成 */
export function todayGroups(now = state.now) {
  const items = tasksOnDay(now);
  return {
    overdue: items.filter((t) => !t.done && isOverdue(t, now)),
    pending: items.filter((t) => !t.done && !isOverdue(t, now)),
    done: items.filter((t) => t.done),
  };
}

/** 小部件用：接下来要做的事（含逾期），最多 n 条 */
export function upcoming(now = state.now, n = 6): Task[] {
  return state.tasks
    .filter((t) => !t.done && t.dueAt)
    .sort((a, b) => {
      const ao = isOverdue(a, now) ? 0 : 1;
      const bo = isOverdue(b, now) ? 0 : 1;
      return ao - bo || byDue(a, b);
    })
    .slice(0, n);
}

/** 区间内按天分组的任务（只返回有任务的日子），用于小部件的周/月视图 */
export function tasksByDayBetween(from: Date, to: Date): { date: Date; tasks: Task[] }[] {
  const start = startOfDay(from).getTime();
  const end = startOfDay(to).getTime() + 86_400_000;
  const groups = new Map<string, Task[]>();
  for (const t of state.tasks) {
    if (!t.dueAt) continue;
    const d = fromStamp(t.dueAt);
    if (!d || d.getTime() < start || d.getTime() >= end) continue;
    const key = dayKey(d);
    const list = groups.get(key);
    if (list) list.push(t);
    else groups.set(key, [t]);
  }
  return [...groups.entries()]
    .map(([key, tasks]) => ({
      date: new Date(`${key}T00:00:00`),
      tasks: tasks.sort(byDue),
    }))
    .sort((a, b) => a.date.getTime() - b.date.getTime());
}

/** 某天完成情况，用于分组标题右侧的 x/y */
export function dayProgress(tasks: Task[]): string {
  const done = tasks.filter((t) => t.done).length;
  return `${done}/${tasks.length}`;
}

/** 有任务的日期集合，供月历画圆点 */
export function taskDays(): Map<string, Task[]> {
  const map = new Map<string, Task[]>();
  for (const t of state.tasks) {
    if (!t.dueAt) continue;
    const d = fromStamp(t.dueAt)!;
    const key = dayKey(d);
    const list = map.get(key);
    if (list) list.push(t);
    else map.set(key, [t]);
  }
  return map;
}

export function todayProgress(now = state.now) {
  const items = tasksOnDay(now);
  const done = items.filter((t) => t.done).length;
  return { done, total: items.length };
}
