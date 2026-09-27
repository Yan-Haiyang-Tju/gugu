<script setup lang="ts">
// 桌面小部件：钉在壁纸层常驻显示，不抢焦点。
// 交互克制——点击任务行会唤出面板去编辑，避免在小部件里塞复杂表单。
import { computed, ref } from "vue";
import { api, type Task } from "../lib/api";
import { state, todayGroups, todayProgress, upcoming } from "../lib/store";
import { fromStamp, humanDay, weekdayCN } from "../lib/time";
import MiniMonth from "./MiniMonth.vue";
import TaskRow from "./TaskRow.vue";

const view = ref(new Date(state.now.getFullYear(), state.now.getMonth(), 1));

const monthLabel = computed(
  () => `${view.value.getFullYear()}年${view.value.getMonth() + 1}月`,
);
const todayLabel = computed(
  () =>
    `今天 · ${state.now.getMonth() + 1}月${state.now.getDate()}日 周${weekdayCN(state.now)}`,
);
const progress = computed(() => todayProgress());
const groups = computed(() => todayGroups());

/** 今天没安排时，顺势展示接下来最近的几条，避免小部件空着 */
const fallback = computed(() => upcoming(state.now, 5));
const showFallback = computed(
  () => groups.value.overdue.length + groups.value.pending.length === 0,
);

const list = computed<Task[]>(() =>
  showFallback.value
    ? fallback.value
    : [...groups.value.overdue, ...groups.value.pending],
);

const shiftMonth = (n: number) => {
  view.value = new Date(view.value.getFullYear(), view.value.getMonth() + n, 1);
};

const footText = computed(() => {
  const { done, total } = progress.value;
  if (total === 0) return "今天还没有安排";
  if (done === total) return "都完成啦，今天很老实 ✓";
  return `今天 ${done}/${total} 完成`;
});

/**
 * 拖动窗口。
 *
 * 钉到壁纸层后窗口变成子窗口，系统的原生拖动（data-tauri-drag-region）不再生效，
 * 所以这里自己跟踪指针位移、按物理像素上报，由 Rust 侧统一换算成屏幕坐标摆放。
 * 不管钉没钉住都走这一条路径，行为一致。
 */
function startDrag(e: PointerEvent) {
  if (e.button !== 0) return;
  const dpr = window.devicePixelRatio || 1;
  let lastX = e.screenX;
  let lastY = e.screenY;
  let dx = 0;
  let dy = 0;
  let raf = 0;

  // 合并到每一帧发一次，避免拖动时把 IPC 打满
  const flush = () => {
    raf = 0;
    if (dx || dy) {
      void api.nudgeWidget(dx, dy);
      dx = 0;
      dy = 0;
    }
  };
  const move = (ev: PointerEvent) => {
    dx += Math.round((ev.screenX - lastX) * dpr);
    dy += Math.round((ev.screenY - lastY) * dpr);
    lastX = ev.screenX;
    lastY = ev.screenY;
    if (!raf) raf = requestAnimationFrame(flush);
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    if (raf) cancelAnimationFrame(raf);
    flush();
    void api.saveWidgetPos();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}
</script>

<template>
  <div class="win widget">
    <div class="w-head drag" @pointerdown="startDrag">
      <span class="brand">咕咕<i /></span>
      <span class="w-nav">
        <button class="ghost" title="上个月" @click="shiftMonth(-1)">‹</button>
        <b>{{ monthLabel }}</b>
        <button class="ghost" title="下个月" @click="shiftMonth(1)">›</button>
      </span>
    </div>

    <MiniMonth :year="view.getFullYear()" :month0="view.getMonth()" />

    <div class="w-div" />

    <div class="w-today-head">
      <b>{{ todayLabel }}</b>
      <span class="w-count">{{ progress.done }}/{{ progress.total }}</span>
    </div>

    <div class="w-list">
      <template v-if="list.length">
        <div v-for="t in list" :key="t.id" class="w-item">
          <span v-if="showFallback" class="w-day">{{ humanDay(fromStamp(t.dueAt)!) }}</span>
          <TaskRow :task="t" @open="api.showPanel()" />
        </div>
      </template>
      <p v-else class="w-empty">今天没有待办，双击 Ctrl 添加</p>
    </div>

    <div class="w-foot">
      <button class="w-add" title="新建任务" @click="api.showPanelAdd()">＋ 新建</button>
      <span class="w-tip">{{ footText }}</span>
      <button class="ghost" title="设置" @click="api.showPanelSettings()">⚙</button>
    </div>
  </div>
</template>

<style scoped>
.widget {
  padding: 14px 14px 8px;
  gap: 0;
}

.w-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  padding: 2px 2px 0;
}

.brand {
  font-weight: 700;
  font-size: 15px;
  letter-spacing: 0.5px;
  display: flex;
  align-items: center;
}

.brand i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  display: inline-block;
  margin-left: 5px;
}

.w-nav {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: 12px;
  color: var(--text-2);
}

.w-nav b {
  font-weight: 500;
  min-width: 68px;
  text-align: center;
  color: var(--text-1);
  font-variant-numeric: tabular-nums;
}

.w-div {
  height: 1px;
  background: var(--hairline);
  margin: 10px -14px;
}

.w-today-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 12.5px;
  padding: 0 2px 6px;
}

.w-today-head b {
  font-weight: 600;
}

.w-count {
  color: var(--text-3);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.w-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  margin: 0 -4px;
  padding: 0 4px;
}

.w-item {
  position: relative;
}

.w-day {
  display: block;
  font-size: 10px;
  color: var(--text-3);
  padding: 6px 6px 0;
}

.w-empty {
  font-size: 11.5px;
  color: var(--text-3);
  padding: 10px 6px;
}

.w-foot {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 0 2px;
  border-top: 1px solid var(--hairline);
  margin: 6px -14px 0;
  padding-left: 14px;
  padding-right: 12px;
  font-size: 10.5px;
  color: var(--text-3);
}

.w-add {
  font-size: 11px;
  color: var(--accent-deep);
  padding: 2px 6px;
  border-radius: 6px;
  transition: background 0.15s;
}

[data-mode="dark"] .w-add {
  color: var(--accent);
}

.w-add:hover {
  background: var(--accent-weak);
}

.w-tip {
  flex: 1;
  text-align: right;
}
</style>
