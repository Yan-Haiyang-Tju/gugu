// 时间工具。全程使用本地时间，字符串格式与 Rust 侧保持一致：YYYY-MM-DDTHH:MM:SS

export const pad = (n: number) => String(n).padStart(2, "0");

export const WEEK_CN = ["日", "一", "二", "三", "四", "五", "六"];

/** Date → "YYYY-MM-DDTHH:MM:SS" */
export function toStamp(d: Date): string {
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
    `T${pad(d.getHours())}:${pad(d.getMinutes())}:00`
  );
}

/** "YYYY-MM-DDTHH:MM:SS" → Date（解析失败返回 null） */
export function fromStamp(s: string | null | undefined): Date | null {
  if (!s) return null;
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** "YYYY-MM-DD"，用作按天分组的键 */
export function dayKey(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function keyOfStamp(s: string | null | undefined): string | null {
  const d = fromStamp(s);
  return d ? dayKey(d) : null;
}

export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

export function addDays(d: Date, n: number): Date {
  const r = new Date(d);
  r.setDate(r.getDate() + n);
  return r;
}

export function startOfDay(d: Date): Date {
  const r = new Date(d);
  r.setHours(0, 0, 0, 0);
  return r;
}

/** 以周一为一周的第一天 */
export function startOfWeek(d: Date): Date {
  const r = startOfDay(d);
  const shift = (r.getDay() + 6) % 7;
  return addDays(r, -shift);
}

export function addMonths(d: Date, n: number): Date {
  const r = new Date(d.getFullYear(), d.getMonth() + n, 1);
  return r;
}

/** 该月最后一天 */
export function endOfMonth(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth() + 1, 0);
}

/** "16:30" */
export function fmtHM(d: Date): string {
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function weekdayCN(d: Date): string {
  return WEEK_CN[d.getDay()];
}

/** 今天 / 明天 / 昨天 / 9月27日 周日 */
export function humanDay(d: Date, now = new Date()): string {
  const diff = Math.round(
    (startOfDay(d).getTime() - startOfDay(now).getTime()) / 86400000,
  );
  if (diff === 0) return "今天";
  if (diff === 1) return "明天";
  if (diff === 2) return "后天";
  if (diff === -1) return "昨天";
  return `${d.getMonth() + 1}月${d.getDate()}日 周${weekdayCN(d)}`;
}

/** 距离现在多久，用于任务的相对时间提示 */
export function relative(d: Date, now = new Date()): string {
  const mins = Math.round((d.getTime() - now.getTime()) / 60000);
  const abs = Math.abs(mins);
  const suffix = mins < 0 ? "前" : "后";
  if (abs < 1) return "就是现在";
  if (abs < 60) return `${abs} 分钟${suffix}`;
  if (abs < 60 * 24) return `${Math.round(abs / 60)} 小时${suffix}`;
  return `${Math.round(abs / 1440)} 天${suffix}`;
}

/** 一天中的第几分钟，用于时间轴定位 */
export function minutesOfDay(d: Date): number {
  return d.getHours() * 60 + d.getMinutes();
}

/** 月历网格：以周一开头，补齐前后使行数完整 */
export function monthGrid(year: number, month0: number) {
  const first = new Date(year, month0, 1);
  const start = (first.getDay() + 6) % 7;
  const daysInMonth = new Date(year, month0 + 1, 0).getDate();
  const prevDays = new Date(year, month0, 0).getDate();
  const cells: { date: Date; out: boolean }[] = [];
  for (let i = start - 1; i >= 0; i--) {
    cells.push({ date: new Date(year, month0 - 1, prevDays - i), out: true });
  }
  for (let d = 1; d <= daysInMonth; d++) {
    cells.push({ date: new Date(year, month0, d), out: false });
  }
  let next = 1;
  while (cells.length % 7 !== 0) {
    cells.push({ date: new Date(year, month0 + 1, next++), out: true });
  }
  return cells;
}
