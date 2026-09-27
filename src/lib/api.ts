// 与 Rust 后端通信的唯一出口。所有 invoke 都收在这里，前端组件不直接碰命令名。

import { invoke } from "@tauri-apps/api/core";

export type Category = "work" | "study" | "life" | "health" | "social" | "errand";
export type ThemeName = "celadon" | "azure" | "tangerine";

/** 分类的中文名与色签变量（与 tokens.css 中的 --c-* 对应） */
export const CATEGORIES: { key: Category; label: string; color: string }[] = [
  { key: "work", label: "工作", color: "var(--c-work)" },
  { key: "study", label: "学习", color: "var(--c-study)" },
  { key: "life", label: "生活", color: "var(--c-life)" },
  { key: "health", label: "健康", color: "var(--c-health)" },
  { key: "social", label: "社交", color: "var(--c-social)" },
  { key: "errand", label: "杂务", color: "var(--c-errand)" },
];

export const categoryOf = (key: string) =>
  CATEGORIES.find((c) => c.key === key) ?? CATEGORIES[5];

export interface Task {
  id: number;
  title: string;
  notes: string;
  /** "YYYY-MM-DDTHH:MM:SS"，null 表示无截止时间 */
  dueAt: string | null;
  category: Category;
  /** 0 普通 / 1 重要 */
  priority: number;
  done: boolean;
  doneAt: string | null;
  remindOffsets: number[];
  repeatRule: string | null;
  createdAt: string;
}

export interface TaskInput {
  id?: number | null;
  title: string;
  notes?: string;
  dueAt?: string | null;
  category?: Category;
  priority?: number;
  remindOffsets?: number[];
  repeatRule?: string | null;
}

/** 快速添加解析出的预填内容，交给编辑小窗继续确认 */
export interface TaskPrefill {
  title?: string;
  dueAt?: string | null;
  category?: Category | null;
  priority?: number;
  repeatRule?: string | null;
}

export interface Settings {
  theme: ThemeName;
  mode: "light" | "dark";
  glassAlpha: number;
  blur: boolean;
  widgetVisible: boolean;
  pinDesktop: boolean;
  clickThrough: boolean;
  hotkeyEnabled: boolean;
  hotkeyWindowMs: number;
  dnd: boolean;
  defaultRemindOffsets: number[];
  widgetX: number;
  widgetY: number;
  widgetW: number;
  widgetH: number;
  autostart: boolean;
}

export interface Tmux {
  /** 面板需要切到哪个视图 */
  panel?: "today" | "week" | "month" | "settings";
  /** 是否把焦点放到快速添加框 */
  quickAdd?: boolean;
}

export const api = {
  // ---- 任务 ----
  listTasks: () => invoke<Task[]>("list_tasks"),
  saveTask: (task: TaskInput) => invoke<Task>("save_task", { task }),
  setDone: (id: number, done: boolean) => invoke<void>("set_done", { id, done }),
  deleteTask: (id: number) => invoke<void>("delete_task", { id }),

  // ---- 设置 ----
  getSettings: () => invoke<Settings>("get_settings"),
  /** 系统级磨砂是否可用（小部件钉成子窗口后，这个能力不一定有） */
  blurActive: () => invoke<boolean>("blur_active"),
  setSettings: (patch: Partial<Settings>) => invoke<Settings>("set_settings", { patch }),

  // ---- 窗口 ----
  showPanel: () => invoke<void>("show_panel_cmd"),
  showPanelAdd: () => invoke<void>("show_panel_add"),
  showPanelSettings: () => invoke<void>("show_panel_settings"),
  hidePanel: () => invoke<void>("hide_panel_cmd"),
  togglePanel: () => invoke<void>("toggle_panel_cmd"),
  saveWidgetPos: () => invoke<void>("save_widget_pos"),
  nudgeWidget: (dx: number, dy: number) => invoke<void>("nudge_widget", { dx, dy }),
  resetWidgetPos: () => invoke<void>("reset_widget_pos"),

  // ---- 系统 ----
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (on: boolean) => invoke<boolean>("set_autostart", { on }),
  testNotification: () => invoke<void>("test_notification"),
  exportData: () => invoke<string>("export_data"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  dataDir: () => invoke<string>("data_dir"),
  quit: () => invoke<void>("quit_app"),
};
