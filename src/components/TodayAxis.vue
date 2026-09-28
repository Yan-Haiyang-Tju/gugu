<script setup lang="ts">
// 今天视图：纵向时间轴，任务卡按截止时间定位，「现在」是一条带脉冲的红线。
import { computed } from "vue";
import { categoryOf, type Task } from "../lib/api";
import { isOverdue, postpone, state, todayGroups, toggleDone } from "../lib/store";
import { fmtHM, fromStamp, minutesOfDay } from "../lib/time";

const emit = defineEmits<{ (e: "edit", task: Task): void }>();

const DAY_START = 7 * 60; // 07:00
const DAY_END = 23 * 60; // 23:00
const PX = 0.62; // 每分钟像素数 ≈ 37px/小时

const groups = computed(() => todayGroups());
const untimed = computed(() => state.tasks.filter((t) => !t.dueAt && !t.done));

const axisHeight = (DAY_END - DAY_START) * PX;

const hours = computed(() => {
  const out: number[] = [];
  for (let h = DAY_START / 60; h <= DAY_END / 60; h += 1) out.push(h);
  return out;
});

function topOf(task: Task): number {
  const d = fromStamp(task.dueAt);
  if (!d) return 0;
  const m = Math.max(DAY_START, Math.min(DAY_END, minutesOfDay(d)));
  return (m - DAY_START) * PX;
}

const nowMinutes = computed(() => state.now.getHours() * 60 + state.now.getMinutes());
const nowTop = computed(() =>
  Math.max(0, Math.min(axisHeight, (nowMinutes.value - DAY_START) * PX)),
);
const nowVisible = computed(() => nowMinutes.value >= DAY_START && nowMinutes.value <= DAY_END);
const nowLabel = computed(() => `现在 ${fmtHM(state.now)}`);

/** 每行显示：逾期置顶，其余按时间轴走 */
const rows = computed(() => [
  ...groups.value.overdue,
  ...groups.value.pending,
  ...groups.value.done,
]);

/** 时间轴上有卡片的任务（无截止时间的单独列出） */
const timed = computed(() => rows.value.filter((t) => t.dueAt));
</script>

<template>
  <div class="wrap">
    <div class="axis" :style="{ height: axisHeight + 32 + 'px' }">
      <div v-for="h in hours" :key="h" class="hour" :style="{ top: (h * 60 - DAY_START) * PX + 8 + 'px' }">
        <b>{{ String(h).padStart(2, "0") }}:00</b>
      </div>

      <div v-if="nowVisible" class="nowline" :style="{ top: nowTop + 8 + 'px' }">
        <i /><b>{{ nowLabel }}</b>
      </div>

      <div
        v-for="t in timed"
        :key="t.id"
        class="trow"
        :class="{ done: t.done, overdue: isOverdue(t), later: t.done }"
        :style="{ top: topOf(t) + 6 + 'px' }"
      >
        <div class="card" @click="emit('edit', t)">
          <span class="ctime">{{ fmtHM(fromStamp(t.dueAt)!) }}</span>
          <span class="ctitle">{{ t.title }}</span>
          <span v-if="t.priority" class="pri" title="重要">!</span>
          <span class="tagpill" :style="{ '--tc': categoryOf(t.category).color }">
            {{ categoryOf(t.category).label }}
          </span>
          <span v-if="isOverdue(t)" class="pill-over">已逾期</span>
          <span class="act">
            <button class="mini-btn" title="顺延到明天" @click.stop="postpone(t)">推到明天</button>
            <button class="mini-btn ok" @click.stop="toggleDone(t, true)">完成</button>
          </span>
          <button class="ck" :class="{ on: t.done }" @click.stop="toggleDone(t)">
            <svg viewBox="0 0 12 10"><path d="M1 5.2 4.2 8.4 11 1.6" /></svg>
          </button>
        </div>
      </div>
    </div>

    <div v-if="untimed.length" class="untimed">
      <b>未排期</b>
      <div v-for="t in untimed" :key="t.id" class="urow" @click="emit('edit', t)">
        <span class="ctitle">{{ t.title }}</span>
        <span class="tagpill" :style="{ '--tc': categoryOf(t.category).color }">
          {{ categoryOf(t.category).label }}
        </span>
        <button class="ck" @click.stop="toggleDone(t)">
          <svg viewBox="0 0 12 10"><path d="M1 5.2 4.2 8.4 11 1.6" /></svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  padding: 4px 0 16px;
}

.axis {
  position: relative;
}

.hour {
  position: absolute;
  left: 52px;
  right: 8px;
  height: 1px;
  background: var(--hairline);
}

.hour b {
  position: absolute;
  left: -46px;
  top: -6px;
  width: 38px;
  text-align: right;
  font-size: 10px;
  font-weight: 400;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.nowline {
  position: absolute;
  left: 52px;
  right: 8px;
  height: 0;
  border-top: 1.5px solid var(--danger);
  z-index: 3;
}

.nowline b {
  position: absolute;
  left: -50px;
  top: -9px;
  width: 46px;
  text-align: center;
  font-size: 10px;
  font-weight: 600;
  color: var(--danger);
  background: var(--solid);
  border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
  padding: 1px 0;
  border-radius: 10px;
  font-variant-numeric: tabular-nums;
}

.nowline i {
  position: absolute;
  left: -3.5px;
  top: -3.5px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--danger);
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 45%, transparent);
  }
  70% {
    box-shadow: 0 0 0 7px transparent;
  }
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
}

.trow {
  position: absolute;
  left: 60px;
  right: 8px;
}

.card {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 30px;
  padding: 6px 10px;
  border-radius: 10px;
  background: var(--surface-2);
  border: 1px solid var(--hairline);
  border-left: 2px solid transparent;
  transition: border-color 0.15s, background 0.15s;
}

.card:hover {
  background: color-mix(in srgb, var(--accent) 6%, var(--surface-2));
}

.ctime {
  width: 36px;
  flex: none;
  font-size: 11px;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
}

.ctitle {
  flex: 1;
  min-width: 0;
  font-size: 12.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pri {
  flex: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 10px;
  font-weight: 700;
  background: color-mix(in srgb, var(--danger) 15%, transparent);
  color: var(--danger);
}

.act {
  display: none;
  gap: 5px;
  flex: none;
}

.card:hover .act {
  display: flex;
}

.trow.overdue .ctime {
  color: var(--danger);
  font-weight: 600;
}

.trow.overdue .card {
  border-left-color: var(--danger);
}

.trow.done .card {
  opacity: 0.5;
}

.trow.done .ctitle {
  color: var(--text-3);
  text-decoration: line-through;
  text-decoration-color: var(--text-3);
}

.trow.done .ctime {
  color: var(--text-3);
}

.untimed {
  margin: 18px 8px 0;
  padding-top: 10px;
  border-top: 1px dashed var(--hairline);
}

.untimed > b {
  display: block;
  font-size: 11px;
  color: var(--text-3);
  font-weight: 500;
  padding: 0 6px 4px;
}

.urow {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 6px;
  border-radius: 8px;
  transition: background 0.15s;
}

.urow:hover {
  background: var(--surface-2);
}
</style>
