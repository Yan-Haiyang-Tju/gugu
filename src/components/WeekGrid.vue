<script setup lang="ts">
// 周视图：7 列时间网格，任务块按截止时间落格，点块直接编辑。
import { computed } from "vue";
import { categoryOf, type Task } from "../lib/api";
import { isOverdue, state } from "../lib/store";
import { addDays, dayKey, fmtHM, fromStamp, minutesOfDay, startOfWeek, weekdayCN } from "../lib/time";

const props = defineProps<{ anchor: Date }>();
const emit = defineEmits<{ (e: "edit", task: Task): void }>();

const DAY_START = 7 * 60;
const DAY_END = 23 * 60;
const PX = 0.62;

const weekStart = computed(() => startOfWeek(props.anchor));
const days = computed(() =>
  Array.from({ length: 7 }, (_, i) => addDays(weekStart.value, i)),
);
const todayKey = computed(() => dayKey(state.now));

const hours = computed(() => {
  const out: number[] = [];
  for (let h = DAY_START / 60; h <= DAY_END / 60; h += 1) out.push(h);
  return out;
});

const axisHeight = (DAY_END - DAY_START) * PX;

/** 每天的任务，按时间排好 */
const byDay = computed(() => {
  const map = new Map<string, Task[]>();
  for (const t of state.tasks) {
    if (!t.dueAt) continue;
    const key = dayKey(fromStamp(t.dueAt)!);
    const list = map.get(key);
    if (list) list.push(t);
    else map.set(key, [t]);
  }
  for (const list of map.values()) {
    list.sort(
      (a, b) =>
        (fromStamp(a.dueAt)?.getTime() ?? 0) - (fromStamp(b.dueAt)?.getTime() ?? 0),
    );
  }
  return map;
});

const tasksOf = (d: Date): Task[] => byDay.value.get(dayKey(d)) ?? [];

function topOf(task: Task): number {
  const d = fromStamp(task.dueAt);
  if (!d) return 0;
  const m = Math.max(DAY_START, Math.min(DAY_END, minutesOfDay(d)));
  return (m - DAY_START) * PX;
}

const rangeLabel = computed(
  () =>
    `${weekStart.value.getMonth() + 1}月${weekStart.value.getDate()}日 – ` +
    `${days.value[6].getMonth() + 1}月${days.value[6].getDate()}日`,
);
</script>

<template>
  <div class="week">
    <div class="wk-range">{{ rangeLabel }}</div>

    <div class="wk-head">
      <span class="wk-gutter" />
      <div
        v-for="d in days"
        :key="d.toISOString()"
        class="wk-day"
        :class="{ today: dayKey(d) === todayKey }"
      >
        <b>周{{ weekdayCN(d) }}</b>
        <i>{{ d.getDate() }}</i>
      </div>
    </div>

    <div class="wk-body" :style="{ height: axisHeight + 16 + 'px' }">
      <div
        v-for="h in hours"
        :key="h"
        class="wl"
        :style="{ top: (h * 60 - DAY_START) * PX + 'px' }"
      >
        <b>{{ String(h).padStart(2, "0") }}</b>
      </div>

      <div class="wk-cols">
        <div v-for="d in days" :key="d.toISOString()" class="wk-col">
          <button
            v-for="t in tasksOf(d)"
            :key="t.id"
            class="wk-chip"
            :class="{ fin: t.done, over: isOverdue(t) }"
            :style="{
              '--tc': categoryOf(t.category).color,
              top: topOf(t) + 'px',
            }"
            :title="`${fmtHM(fromStamp(t.dueAt)!)} ${t.title}`"
            @click="emit('edit', t)"
          >
            <em>{{ fmtHM(fromStamp(t.dueAt)!) }}</em> {{ t.title }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.week {
  padding: 6px 4px 12px;
}

.wk-range {
  font-size: 11.5px;
  color: var(--text-2);
  padding: 2px 4px 8px;
  font-variant-numeric: tabular-nums;
}

.wk-head {
  display: grid;
  grid-template-columns: 44px repeat(7, 1fr);
  padding-bottom: 4px;
}

.wk-day {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
  font-size: 11px;
  color: var(--text-2);
  padding: 2px 0;
  border-radius: 8px;
  transition: background 0.15s;
}

.wk-day:hover {
  background: var(--surface-2);
}

.wk-day b {
  font-weight: 500;
}

.wk-day i {
  font-style: normal;
  font-size: 10.5px;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.wk-day.today {
  color: var(--accent-deep);
}

.wk-day.today b,
.wk-day.today i {
  font-weight: 600;
}

[data-mode="dark"] .wk-day.today {
  color: var(--accent);
}

.wk-body {
  position: relative;
}

.wl {
  position: absolute;
  left: 44px;
  right: 0;
  height: 1px;
  background: var(--hairline);
}

.wl b {
  position: absolute;
  left: -40px;
  top: -6px;
  width: 32px;
  text-align: right;
  font-size: 10px;
  font-weight: 400;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.wk-cols {
  position: absolute;
  left: 44px;
  right: 0;
  top: 0;
  bottom: 0;
  display: grid;
  grid-template-columns: repeat(7, 1fr);
}

.wk-col {
  position: relative;
  border-left: 1px solid var(--hairline);
}

.wk-col:last-child {
  border-right: 1px solid var(--hairline);
}

.wk-chip {
  position: absolute;
  left: 3px;
  right: 3px;
  font-size: 10px;
  line-height: 1.3;
  text-align: left;
  padding: 2.5px 5px;
  border-radius: 6px;
  border-left: 2px solid var(--tc);
  background: color-mix(in srgb, var(--tc) 13%, transparent);
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: filter 0.15s;
}

.wk-chip:hover {
  filter: brightness(0.96);
}

.wk-chip em {
  font-style: normal;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
}

.wk-chip.fin {
  opacity: 0.55;
  text-decoration: line-through;
}

.wk-chip.over {
  border-left-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 12%, transparent);
}
</style>
