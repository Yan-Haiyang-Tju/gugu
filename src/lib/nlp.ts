// 快速添加的自然语言解析。
// 目标是把「明天15:00 交周报 #工作 !重要」这类输入拆成结构化字段，
// 解析结果会先以胶囊形式回显，用户确认后才真正建任务。

import type { Category } from "./api";
import { addDays, pad, startOfDay, toStamp } from "./time";

export interface ParsedToken {
  kind: "date" | "time" | "tag" | "priority" | "repeat";
  label: string;
}

export interface Parsed {
  title: string;
  /** "YYYY-MM-DDTHH:MM:SS"，null 表示没解析出时间 */
  dueAt: string | null;
  category: Category | null;
  priority: number;
  repeatRule: string | null;
  tokens: ParsedToken[];
}

const TAG_ALIASES: Record<string, Category> = {
  工作: "work",
  work: "work",
  学习: "study",
  study: "study",
  生活: "life",
  life: "life",
  健康: "health",
  health: "health",
  社交: "social",
  social: "social",
  杂务: "errand",
  errand: "errand",
};

const WEEKDAY_INDEX: Record<string, number> = {
  一: 1, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6, 日: 0, 天: 0,
};

const BYDAY = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

/** 时段词 → 该时段的默认小时 + 是否强制下午 */
const PERIODS: Record<string, { hour: number; pm: boolean | null }> = {
  凌晨: { hour: 5, pm: false },
  早上: { hour: 8, pm: false },
  早晨: { hour: 8, pm: false },
  上午: { hour: 9, pm: false },
  中午: { hour: 12, pm: null },
  下午: { hour: 15, pm: true },
  傍晚: { hour: 18, pm: true },
  晚上: { hour: 20, pm: true },
  夜里: { hour: 21, pm: true },
  今晚: { hour: 20, pm: true },
};

export function parseQuick(raw: string, now = new Date()): Parsed {
  let s = ` ${raw} `;
  const tokens: ParsedToken[] = [];

  let day: Date | null = null; // 已明确的日期
  let hour: number | null = null;
  let minute = 0;
  let periodHint: { hour: number; pm: boolean | null } | null = null;
  let category: Category | null = null;
  let priority = 0;
  let repeatRule: string | null = null;

  // ---- 标签 #工作 ----
  s = s.replace(/#([^\s#!]+)/g, (_m, g: string) => {
    const hit = TAG_ALIASES[g] ?? TAG_ALIASES[g.toLowerCase()];
    if (hit) {
      category = hit;
      tokens.push({ kind: "tag", label: `#${g}` });
      return " ";
    }
    return _m; // 不是已知标签就原样保留在标题里
  });

  // ---- 重要标记 !重要 / ! ----
  s = s.replace(/!([^\s#!]*)/g, (_m, g: string) => {
    priority = 1;
    tokens.push({ kind: "priority", label: g ? `!${g}` : "!重要" });
    return " ";
  });

  // ---- 重复 ----
  s = s.replace(/(每天|每日)/g, () => {
    repeatRule = "FREQ=DAILY";
    tokens.push({ kind: "repeat", label: "每天" });
    return " ";
  });
  s = s.replace(/每(?:周|星期|礼拜)([一二三四五六日天])/g, (_m, w: string) => {
    const idx = WEEKDAY_INDEX[w];
    repeatRule = `FREQ=WEEKLY;BYDAY=${BYDAY[idx]}`;
    tokens.push({ kind: "repeat", label: `每周${w}` });
    // 重复的首次截止 = 下一个该星期几
    if (!day) {
      const base = startOfDay(now);
      let diff = (idx - base.getDay() + 7) % 7;
      if (diff === 0) diff = 7;
      day = addDays(base, diff);
    }
    return " ";
  });
  s = s.replace(/(每周|每星期|每礼拜)/g, () => {
    repeatRule = "FREQ=WEEKLY";
    tokens.push({ kind: "repeat", label: "每周" });
    return " ";
  });

  // ---- 绝对日期：2026-09-30 / 9月30日 / 9/30 ----
  s = s.replace(/(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})/g, (_m, y, mo, d) => {
    day = new Date(Number(y), Number(mo) - 1, Number(d));
    tokens.push({ kind: "date", label: `${mo}月${d}日` });
    return " ";
  });
  s = s.replace(/(\d{1,2})月(\d{1,2})[日号]/g, (_m, mo, d) => {
    let year = now.getFullYear();
    const candidate = new Date(year, Number(mo) - 1, Number(d));
    // 已经过去的日子按明年算
    if (candidate.getTime() < startOfDay(now).getTime()) year += 1;
    day = new Date(year, Number(mo) - 1, Number(d));
    tokens.push({ kind: "date", label: `${mo}月${d}日` });
    return " ";
  });

  // ---- 相对日期 ----
  const RELATIVE: Record<string, number> = {
    大后天: 3, 后天: 2, 明天: 1, 明日: 1, 今天: 0, 今日: 0, 昨天: -1,
  };
  s = s.replace(/(大后天|后天|明天|明日|今天|今日|昨天)/g, (_m, w: string) => {
    day = addDays(startOfDay(now), RELATIVE[w]);
    tokens.push({ kind: "date", label: w });
    return " ";
  });

  // ---- 星期：周三 / 下周三 / 下下周三 ----
  s = s.replace(/(下下|下)?(?:周|星期|礼拜)([一二三四五六日天])/g, (_m, prefix: string, w: string) => {
    const idx = WEEKDAY_INDEX[w];
    const base = startOfDay(now);
    const weekStart = addDays(base, -((base.getDay() + 6) % 7)); // 本周一
    let target = addDays(weekStart, idx === 0 ? 6 : idx - 1);
    if (prefix === "下") target = addDays(target, 7);
    else if (prefix === "下下") target = addDays(target, 14);
    else if (target.getTime() < base.getTime()) target = addDays(target, 7);
    day = target;
    tokens.push({ kind: "date", label: `${prefix ?? ""}周${w}` });
    return " ";
  });

  // ---- 时间：15:30 / 15：30 ----
  s = s.replace(
    /(凌晨|早上|早晨|上午|中午|下午|傍晚|晚上|夜里)?\s*(\d{1,2})[:：](\d{2})/g,
    (_m, p: string | undefined, h: string, mi: string) => {
      hour = Number(h);
      minute = Number(mi);
      if (p) periodHint = PERIODS[p];
      tokens.push({ kind: "time", label: `${pad(hour)}:${pad(minute)}` });
      return " ";
    },
  );

  // ---- 时间：15点 / 15点半 / 15点20分 ----
  s = s.replace(
    /(凌晨|早上|早晨|上午|中午|下午|傍晚|晚上|夜里)?\s*(\d{1,2})\s*点\s*(半|\d{1,2})?分?/g,
    (_m, p: string | undefined, h: string, mi: string | undefined) => {
      hour = Number(h);
      minute = mi === "半" ? 30 : mi ? Number(mi) : 0;
      if (p) periodHint = PERIODS[p];
      tokens.push({ kind: "time", label: `${pad(hour)}:${pad(minute)}` });
      return " ";
    },
  );

  // ---- 只给了时段：晚上 / 下午 ----
  if (hour === null) {
    s = s.replace(/(凌晨|早上|早晨|上午|中午|下午|傍晚|晚上|夜里|今晚)/g, (_m, p: string) => {
      periodHint = PERIODS[p];
      tokens.push({ kind: "time", label: p });
      return " ";
    });
  }

  // ---- 组装 ----
  // 上面所有赋值都发生在 replace 回调里，TS 的流分析看不到（会把这些变量收窄成 null/never），
  // 所以这里用断言把类型收口回声明类型。语义没有问题：运行时它们确实已经被回调赋过值了。
  const dayPick = day as Date | null;
  const hint = periodHint as { hour: number; pm: boolean | null } | null;
  let hh = hour as number | null;
  let mm = minute;

  if (hint) {
    if (hh === null) {
      hh = hint.hour;
      mm = 0;
    } else if (hint.pm === true && hh < 12) {
      hh += 12; // 下午3点 → 15 点
    } else if (hint.pm === false && hh === 12) {
      hh = 0; // 凌晨12点 → 00 点
    }
  }

  let due: Date | null = null;
  if (dayPick && hh !== null) {
    due = new Date(dayPick);
    due.setHours(hh, mm, 0, 0);
  } else if (dayPick) {
    // 只给了日期：今天补下一个整点，其它日子默认上午 9 点
    if (dayPick.getTime() === startOfDay(now).getTime()) {
      due = new Date(now.getTime() + 3600_000);
      due.setMinutes(0, 0, 0);
    } else {
      due = new Date(dayPick);
      due.setHours(9, 0, 0, 0);
    }
    tokens.push({ kind: "time", label: `${pad(due.getHours())}:${pad(due.getMinutes())}` });
  } else if (hh !== null) {
    // 只给了时间：今天该时刻已过则顺延到明天
    due = new Date(now);
    due.setHours(hh, mm, 0, 0);
    if (due.getTime() <= now.getTime()) due = addDays(due, 1);
    tokens.push({
      kind: "date",
      label: due.getTime() - now.getTime() < 86400000 * 1.5 ? "今天" : "明天",
    });
  }

  const title = s.replace(/\s+/g, " ").trim();

  return {
    title,
    dueAt: due ? toStamp(due) : null,
    category,
    priority,
    repeatRule,
    tokens,
  };
}
