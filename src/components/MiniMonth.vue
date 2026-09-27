<script setup lang="ts">
// 月历网格：小部件里用迷你版（mc-*），月视图里用大格版（mo-*），共用同一套网格数据。
import { computed } from "vue";
import { categoryOf, type Task } from "../lib/api";
import { state, taskDays } from "../lib/store";
import { dayKey, monthGrid } from "../lib/time";

const props = withDefaults(
  defineProps<{
    year: number;
    month0: number;
    big?: boolean;
    selected?: string | null;
  }>(),
  { big: false, selected: null },
);

const emit = defineEmits<{ (e: "pick", date: Date): void }>();

const weekLabels = ["一", "二", "三", "四", "五", "六", "日"];
const cells = computed(() => monthGrid(props.year, props.month0));
const byDay = computed(() => taskDays());
const todayKey = computed(() => dayKey(state.now));

const dotsFor = (d: Date): Task[] => (byDay.value.get(dayKey(d)) ?? []).slice(0, 3);
</script>

<template>
  <div :class="big ? 'month-grid' : 'mini-cal'">
    <div v-for="w in weekLabels" :key="w" :class="big ? 'mo-w' : 'mc-w'">{{ w }}</div>
    <button
      v-for="(cell, i) in cells"
      :key="i"
      :class="[
        big ? 'mo-d' : 'mc-d',
        {
          out: cell.out,
          today: dayKey(cell.date) === todayKey,
          sel: big && selected === dayKey(cell.date),
        },
      ]"
      @click="big && !cell.out && emit('pick', cell.date)"
    >
      <b>{{ cell.date.getDate() }}</b>
      <span v-if="dotsFor(cell.date).length" :class="big ? 'mo-dots' : 'mc-dots'">
        <i
          v-for="(t, j) in dotsFor(cell.date)"
          :key="j"
          :style="{ '--tc': categoryOf(t.category).color }"
        />
      </span>
    </button>
  </div>
</template>

<style scoped>
/* ---------- 迷你版（桌面小部件） ---------- */
.mini-cal {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px 0;
}

.mc-w {
  font-size: 11.5px;
  color: var(--text-3);
  text-align: center;
  padding-bottom: 6px;
}

.mc-d {
  height: 35px;
  position: relative;
  text-align: center;
  font-size: 13.5px;
  color: var(--text-1);
}

.mc-d b {
  font-weight: 400;
  display: inline-grid;
  place-items: center;
  min-width: 27px;
  height: 27px;
  padding: 0 3px;
  border-radius: 14px;
  font-variant-numeric: tabular-nums;
}

.mc-d.out b {
  color: var(--text-3);
  opacity: 0.45;
}

.mc-d.today b {
  background: var(--accent);
  color: var(--on-accent);
  font-weight: 600;
}

.mc-dots {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 1px;
  display: flex;
  justify-content: center;
  gap: 2.5px;
}

.mc-dots i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--tc);
}

/* ---------- 大格版（月视图） ---------- */
.month-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
  padding: 10px 2px 6px;
}

.mo-w {
  font-size: 10.5px;
  color: var(--text-3);
  text-align: center;
  padding-bottom: 2px;
}

.mo-d {
  height: 54px;
  border-radius: 10px;
  font-size: 12px;
  position: relative;
  padding: 5px 7px;
  text-align: left;
  background: var(--surface-2);
  border: 1px solid transparent;
  transition: border-color 0.15s, background 0.15s;
}

.mo-d:hover {
  border-color: var(--hairline);
}

.mo-d b {
  font-weight: 400;
  font-variant-numeric: tabular-nums;
}

.mo-d.out {
  opacity: 0.35;
  cursor: default;
}

.mo-d.today {
  box-shadow: inset 0 0 0 1.5px var(--accent);
}

.mo-d.today b {
  color: var(--accent-deep);
  font-weight: 600;
}

[data-mode="dark"] .mo-d.today b {
  color: var(--accent);
}

.mo-d.sel {
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
}

.mo-dots {
  position: absolute;
  left: 7px;
  bottom: 6px;
  display: flex;
  gap: 2.5px;
}

.mo-dots i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--tc);
}
</style>
