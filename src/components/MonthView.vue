<script setup lang="ts">
// 月视图：整月网格 + 选中日期当天列表
import { computed, ref, watch } from "vue";
import { categoryOf, type Task } from "../lib/api";
import { isOverdue, state, toggleDone } from "../lib/store";
import { dayKey, fmtHM, fromStamp, humanDay } from "../lib/time";
import MiniMonth from "./MiniMonth.vue";

const props = defineProps<{ anchor: Date }>();
const emit = defineEmits<{ (e: "edit", task: Task): void }>();

const view = ref(new Date(props.anchor.getFullYear(), props.anchor.getMonth(), 1));
const selected = ref(new Date(props.anchor));

watch(
  () => props.anchor,
  (d) => {
    view.value = new Date(d.getFullYear(), d.getMonth(), 1);
    selected.value = new Date(d);
  },
);

const selectedTasks = computed(() => {
  const key = dayKey(selected.value);
  return state.tasks
    .filter((t) => t.dueAt && dayKey(fromStamp(t.dueAt)!) === key)
    .sort(
      (a, b) =>
        (fromStamp(a.dueAt)?.getTime() ?? 0) - (fromStamp(b.dueAt)?.getTime() ?? 0),
    );
});

const selectedTitle = computed(() => humanDay(selected.value));
</script>

<template>
  <div class="month">
    <div class="mo-nav">
      <button
        class="ghost"
        title="上个月"
        @click="view = new Date(view.getFullYear(), view.getMonth() - 1, 1)"
      >
        ‹
      </button>
      <b>{{ view.getFullYear() }}年{{ view.getMonth() + 1 }}月</b>
      <button
        class="ghost"
        title="下个月"
        @click="view = new Date(view.getFullYear(), view.getMonth() + 1, 1)"
      >
        ›
      </button>
      <span class="sp" />
      <button
        class="mini-btn"
        @click="
          view = new Date();
          selected = new Date();
        "
      >
        回到今天
      </button>
    </div>

    <MiniMonth
      big
      :year="view.getFullYear()"
      :month0="view.getMonth()"
      :selected="dayKey(selected)"
      @pick="(d) => (selected = d)"
    />

    <div class="mo-list">
      <div class="mo-list-head">{{ selectedTitle }}</div>
      <template v-if="selectedTasks.length">
        <div
          v-for="t in selectedTasks"
          :key="t.id"
          class="mo-row"
          :class="{ done: t.done }"
          @click="emit('edit', t)"
        >
          <b>{{ fmtHM(fromStamp(t.dueAt)!) }}</b>
          <span class="mo-title">{{ t.title }}</span>
          <span v-if="isOverdue(t)" class="pill-over">咕咕咕</span>
          <i :style="{ '--tc': categoryOf(t.category).color }">
            {{ categoryOf(t.category).label }}
          </i>
          <button class="ck" :class="{ on: t.done }" @click.stop="toggleDone(t)">
            <svg viewBox="0 0 12 10"><path d="M1 5.2 4.2 8.4 11 1.6" /></svg>
          </button>
        </div>
      </template>
      <p v-else class="mo-empty">这天还没有安排</p>
    </div>
  </div>
</template>

<style scoped>
.month {
  padding-bottom: 12px;
}

.mo-nav {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 2px 0;
  font-size: 12.5px;
}

.mo-nav b {
  font-weight: 600;
  padding: 0 4px;
  font-variant-numeric: tabular-nums;
}

.sp {
  flex: 1;
}

.mo-list {
  padding: 2px 2px 12px;
  border-top: 1px solid var(--hairline);
  margin-top: 4px;
}

.mo-list-head {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  padding: 8px;
}

.mo-row {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 12px;
  padding: 6px 9px;
  border-radius: 8px;
  transition: background 0.15s;
}

.mo-row:hover {
  background: var(--surface-2);
}

.mo-row b {
  font-weight: 400;
  color: var(--text-2);
  font-size: 11px;
  width: 38px;
  flex: none;
  font-variant-numeric: tabular-nums;
}

.mo-title {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mo-row i {
  font-style: normal;
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 20px;
  flex: none;
  background: color-mix(in srgb, var(--tc) 15%, transparent);
  color: color-mix(in srgb, var(--tc) 78%, var(--text-1) 22%);
}

.mo-row.done .mo-title {
  color: var(--text-3);
  text-decoration: line-through;
  text-decoration-color: var(--text-3);
}

.mo-row.done {
  opacity: 0.6;
}

.mo-empty {
  font-size: 11.5px;
  color: var(--text-3);
  padding: 8px 9px;
}
</style>
