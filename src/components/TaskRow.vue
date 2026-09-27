<script setup lang="ts">
// 紧凑任务行：小部件列表、月视图当日列表共用
import { computed } from "vue";
import { categoryOf, type Task } from "../lib/api";
import { isOverdue, postpone, toggleDone } from "../lib/store";
import { fmtHM, fromStamp } from "../lib/time";

const props = defineProps<{ task: Task }>();
const emit = defineEmits<{ (e: "open", task: Task): void }>();

const due = computed(() => fromStamp(props.task.dueAt));
const overdue = computed(() => isOverdue(props.task));
const cat = computed(() => categoryOf(props.task.category));
</script>

<template>
  <div
    class="row"
    :class="{ done: task.done, overdue }"
    @click="emit('open', task)"
  >
    <span class="time">{{ due ? fmtHM(due) : "--:--" }}</span>
    <span class="title">{{ task.title }}</span>
    <span v-if="task.priority" class="pri" title="重要">!</span>
    <span v-if="overdue" class="pill-over">咕咕咕</span>
    <span v-else class="tagpill" :style="{ '--tc': cat.color }">{{ cat.label }}</span>
    <span class="act">
      <button class="mini-btn" @click.stop="postpone(task)">咕了</button>
      <button class="mini-btn ok" @click.stop="toggleDone(task, true)">完成</button>
    </span>
    <button
      class="ck"
      :class="{ on: task.done }"
      :title="task.done ? '标记为未完成' : '标记完成'"
      @click.stop="toggleDone(task)"
    >
      <svg viewBox="0 0 12 10"><path d="M1 5.2 4.2 8.4 11 1.6" /></svg>
    </button>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5.5px 6px;
  border-radius: 8px;
  font-size: 12.5px;
  transition: background 0.15s;
}

.row:hover {
  background: var(--surface-2);
}

.time {
  width: 34px;
  flex: none;
  color: var(--text-2);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}

.title {
  flex: 1;
  min-width: 0;
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

.row:hover .act {
  display: flex;
}

.row.done .title {
  color: var(--text-3);
  text-decoration: line-through;
  text-decoration-color: var(--text-3);
}

.row.done .time {
  color: var(--text-3);
}

.row.done .tagpill,
.row.done .pri {
  opacity: 0.45;
}

.row.overdue .time {
  color: var(--danger);
  font-weight: 600;
}
</style>
