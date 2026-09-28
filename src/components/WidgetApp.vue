<script setup lang="ts">
// 桌面小部件：常驻桌面，可点选日期、切今天/本周/本月三种范围。
// 任务行点开会在面板里打开编辑窗；勾选框直接改完成状态。
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api } from "../lib/api";
import {
  dayProgress,
  state,
  tasksByDayBetween,
  tasksOnDay,
  todayProgress,
} from "../lib/store";
import {
  addDays,
  dayKey,
  endOfMonth,
  humanDay,
  startOfWeek,
  weekdayCN,
} from "../lib/time";
import MiniMonth from "./MiniMonth.vue";
import TaskRow from "./TaskRow.vue";

type Scope = "day" | "week" | "month";

const view = ref(new Date(state.now.getFullYear(), state.now.getMonth(), 1));
const scope = ref<Scope>("day");
const focusDate = ref(new Date(state.now));
/** 按了双击 Ctrl、小部件被升到顶层时才为 true —— 只有这时它才收得到鼠标 */
const operating = ref(false);

/**
 * Esc 收起。
 *
 * 面板里也有一份同样的处理，但两个窗口的焦点是互斥的：用户点了小部件（拖动、
 * 勾任务）之后焦点在小部件上，面板收不到按键——只在面板上挂 Esc 的话，
 * 从操作模式退不出来。
 */
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && operating.value) {
    void api.hidePanel();
  }
}

onMounted(async () => {
  // 主动问一次当前状态：页面重载会丢掉事件期间设过的标志
  try {
    operating.value = await api.widgetOperating();
  } catch {
    operating.value = false;
  }
  await listen<string>("widget:mode", (e) => {
    operating.value = e.payload === "operate";
  });
  window.addEventListener("keydown", onKey);

  // 尺寸/位置变化后记住新值。
  // 注意不能用 startResizeDragging 的返回值来存：它内部是 PostMessage，调用立刻返回，
  // 那时窗口还没开始变，存下来的是旧尺寸。所以统一靠 onResized 事件。
  let saveTimer: number | undefined;
  await getCurrentWindow().onResized(() => {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => void api.saveWidgetPos(), 400);
  });
});

onUnmounted(() => window.removeEventListener("keydown", onKey));

const monthLabel = computed(
  () => `${view.value.getFullYear()}年${view.value.getMonth() + 1}月`,
);
const isToday = computed(() => dayKey(focusDate.value) === dayKey(state.now));

const weekStart = computed(() => startOfWeek(focusDate.value));
const weekEnd = computed(() => addDays(weekStart.value, 6));
const monthStart = computed(
  () => new Date(view.value.getFullYear(), view.value.getMonth(), 1),
);
const monthEnd = computed(() => endOfMonth(view.value));

/** 单日范围（今天或点选的某天） */
const dayList = computed(() => tasksOnDay(focusDate.value));
/** 周/月范围，按天分组 */
const groups = computed(() => {
  if (scope.value === "week") return tasksByDayBetween(weekStart.value, weekEnd.value);
  if (scope.value === "month") return tasksByDayBetween(monthStart.value, monthEnd.value);
  return [];
});

const listTitle = computed(() => {
  if (scope.value === "week") {
    const a = weekStart.value;
    const b = weekEnd.value;
    return `本周 · ${a.getMonth() + 1}月${a.getDate()}日 – ${b.getMonth() + 1}月${b.getDate()}日`;
  }
  if (scope.value === "month") {
    return `${monthStart.value.getFullYear()}年${monthStart.value.getMonth() + 1}月`;
  }
  if (isToday.value) {
    return `今天 · ${state.now.getMonth() + 1}月${state.now.getDate()}日 周${weekdayCN(state.now)}`;
  }
  return humanDay(focusDate.value);
});

const progress = computed(() => todayProgress());
const rangeCount = computed(() =>
  groups.value.reduce((n, g) => n + g.tasks.length, 0),
);

function pickScope(s: Scope) {
  scope.value = s;
  if (s === "day") focusDate.value = new Date(state.now);
}

/** 点日历上的某天 → 切到那一天 */
function pickDate(d: Date) {
  focusDate.value = d;
  scope.value = "day";
}

function shiftMonth(n: number) {
  view.value = new Date(view.value.getFullYear(), view.value.getMonth() + n, 1);
}

const footText = computed(() => {
  const { done, total } = progress.value;
  if (total === 0) return "今天还没有安排";
  if (done === total) return "都完成啦 ✓";
  return `今天 ${done}/${total}`;
});

/**
 * 拖动窗口。
 *
 * 只有操作模式（按了双击 Ctrl、窗口已升到顶层）才拖得动——沉在桌面层时
 * 窗口根本收不到鼠标。这里直接用系统的原生拖动：它会进入 Windows 自己的
 * 移动循环，比手动跟踪指针位移稳得多（手动方案在移动窗口的过程中会打断
 * 浏览器的指针事件序列，拖两下就断）。
 */
async function startDrag(e: PointerEvent) {
  if (e.button !== 0 || !operating.value) return;
  await getCurrentWindow().startDragging();
  // 移动循环结束后才返回，此时把新位置记下来
  void api.saveWidgetPos();
}

/** 右下角抓手：拖动改变窗口大小（同样只在操作模式下可用） */
async function startResize(e: PointerEvent) {
  if (e.button !== 0 || !operating.value) return;
  e.preventDefault();
  await getCurrentWindow().startResizeDragging("SouthEast");
  void api.saveWidgetPos(); // 尺寸一并记下来，下次启动沿用
}
</script>

<template>
  <div class="win widget" :class="{ operating }">
    <div class="w-head drag" @pointerdown="startDrag">
      <span class="brand">咕咕<i /></span>
      <span class="w-nav">
        <button class="ghost" title="上个月" @click="shiftMonth(-1)">‹</button>
        <b>{{ monthLabel }}</b>
        <button class="ghost" title="下个月" @click="shiftMonth(1)">›</button>
      </span>
    </div>

    <MiniMonth
      :year="view.getFullYear()"
      :month0="view.getMonth()"
      :selected="dayKey(focusDate)"
      @pick="pickDate"
    />

    <div class="w-div" />

    <div class="w-tabs">
      <button :class="{ on: scope === 'day' && isToday }" @click="pickScope('day')">今天</button>
      <button :class="{ on: scope === 'week' }" @click="pickScope('week')">本周</button>
      <button :class="{ on: scope === 'month' }" @click="pickScope('month')">本月</button>
      <span class="sp" />
      <span class="w-count">{{ scope === "day" ? `${progress.done}/${progress.total}` : rangeCount }}</span>
    </div>

    <div class="w-list">
      <div class="w-title">
        <b>{{ listTitle }}</b>
        <button v-if="scope === 'day' && !isToday" class="w-back" @click="pickScope('day')">
          回到今天
        </button>
      </div>

      <!-- 单日：一条平铺列表 -->
      <template v-if="scope === 'day'">
        <TaskRow
          v-for="t in dayList"
          :key="t.id"
          :task="t"
          @open="api.openTask(t.id)"
        />
        <p v-if="!dayList.length" class="w-empty">这天没有安排，双击 Ctrl 添加</p>
      </template>

      <!-- 周 / 月：按天分组 -->
      <template v-else>
        <div v-for="g in groups" :key="dayKey(g.date)" class="w-group">
          <div class="w-group-head">
            <b>{{ g.date.getMonth() + 1 }}月{{ g.date.getDate() }}日 周{{ weekdayCN(g.date) }}</b>
            <span>{{ dayProgress(g.tasks) }}</span>
          </div>
          <TaskRow
            v-for="t in g.tasks"
            :key="t.id"
            :task="t"
            @open="api.openTask(t.id)"
          />
        </div>
        <p v-if="!groups.length" class="w-empty">
          {{ scope === "week" ? "本周没有安排" : "本月没有安排" }}
        </p>
      </template>
    </div>

    <div class="w-foot">
      <button class="w-add" title="新建任务" @click="api.showPanelAdd()">＋ 新建</button>
      <span class="w-tip">{{ operating ? "可拖动 / 右下角缩放 · Esc 收起" : footText }}</span>
      <button class="ghost" title="设置" @click="api.showPanelSettings()">⚙</button>
    </div>

    <!-- 缩放抓手：只在操作模式出现，提示「这里可以拉」 -->
    <span v-if="operating" class="w-resize" title="拖动改变大小" @pointerdown="startResize" />
  </div>
</template>

<style scoped>
.widget {
  padding: 16px 16px 10px;
  gap: 0;
}

/* 右下角缩放手柄：只在操作模式出现 */
.w-resize {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 18px;
  height: 18px;
  cursor: nwse-resize;
  z-index: 5;
}

.w-resize::before {
  content: "";
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 10px;
  height: 10px;
  border-right: 2px solid var(--text-3);
  border-bottom: 2px solid var(--text-3);
  border-bottom-right-radius: 3px;
  opacity: 0.55;
  transition: opacity 0.15s;
}

.w-resize:hover::before {
  opacity: 1;
  border-color: var(--accent);
}

/* 操作模式（按了双击 Ctrl、已升到顶层）时给一圈强调色描边，
   让用户一眼看出「现在可以点、可以拖」 */
.widget.operating {
  box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--accent) 60%, transparent);
}

.w-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  padding: 2px 2px 0;
}

.brand {
  font-weight: 700;
  font-size: 16px;
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
  font-size: 12.5px;
  color: var(--text-2);
}

.w-nav b {
  font-weight: 500;
  min-width: 72px;
  text-align: center;
  color: var(--text-1);
  font-variant-numeric: tabular-nums;
}

.w-div {
  height: 1px;
  background: var(--hairline);
  margin: 12px -16px 0;
}

/* ---------- 范围切换 ---------- */
.w-tabs {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 9px 2px 7px;
}

.w-tabs button {
  font-size: 12.5px;
  color: var(--text-2);
  padding: 3.5px 11px;
  border-radius: 13px;
  transition: background 0.15s, color 0.15s;
}

.w-tabs button:hover {
  background: var(--surface-2);
}

.w-tabs button.on {
  background: var(--accent-weak);
  color: var(--accent-deep);
  font-weight: 600;
}

[data-mode="dark"] .w-tabs button.on {
  color: var(--accent);
}

.w-tabs .sp {
  flex: 1;
}

.w-count {
  color: var(--text-3);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  padding-right: 2px;
}

/* ---------- 列表 ---------- */
.w-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  margin: 0 -6px;
  padding: 0 6px;
}

.w-title {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 2px 2px 4px;
}

.w-title b {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
}

.w-back {
  font-size: 11px;
  color: var(--accent-deep);
  padding: 1px 6px;
  border-radius: 6px;
}

[data-mode="dark"] .w-back {
  color: var(--accent);
}

.w-back:hover {
  background: var(--accent-weak);
}

.w-group {
  padding-bottom: 4px;
}

.w-group-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 7px 8px 1px;
}

.w-group-head b {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--text-3);
}

.w-group-head span {
  font-size: 11px;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.w-empty {
  font-size: 12.5px;
  color: var(--text-3);
  padding: 12px 8px;
}

/* ---------- 底栏 ---------- */
.w-foot {
  display: flex;
  align-items: center;
  gap: 6px;
  border-top: 1px solid var(--hairline);
  margin: 8px -16px 0;
  padding: 8px 14px 2px 16px;
  font-size: 11.5px;
  color: var(--text-3);
}

.w-add {
  font-size: 12px;
  color: var(--accent-deep);
  padding: 3px 7px;
  border-radius: 7px;
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
