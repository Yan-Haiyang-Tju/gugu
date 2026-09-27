<script setup lang="ts">
// 唤出面板：快速添加 + 今天/周/月三视图 + 设置。平时隐藏，双击 Ctrl 唤出。
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, type Task, type TaskPrefill } from "../lib/api";
import { state, todayProgress } from "../lib/store";
import { addDays, dayKey } from "../lib/time";
import MonthView from "./MonthView.vue";
import QuickAdd from "./QuickAdd.vue";
import SettingsView from "./SettingsView.vue";
import TaskEditor from "./TaskEditor.vue";
import TodayAxis from "./TodayAxis.vue";
import WeekGrid from "./WeekGrid.vue";

type Tab = "today" | "week" | "month" | "settings";

const tab = ref<Tab>("today");
const anchor = ref(new Date(state.now));
const editorOpen = ref(false);
const editing = ref<Task | null>(null);
const prefill = ref<TaskPrefill | null>(null);
const quickFocus = ref(0);

const progress = computed(() => todayProgress());
const footLeft = computed(() => {
  const { done, total } = progress.value;
  return total === 0 ? "今天还没有安排" : `今天 ${done}/${total} 完成`;
});

function openEditor(task: Task | null) {
  editing.value = task;
  prefill.value = null;
  editorOpen.value = true;
}

function onQuick(p: TaskPrefill) {
  editing.value = null;
  prefill.value = p;
  editorOpen.value = true;
}

/**
 * 拖动面板。
 *
 * 不用 data-tauri-drag-region：它只认「按下的目标元素本身」带这个属性，
 * 而面板顶部整条都被快速添加框占着，按下去的目标永远是里面的输入框，
 * 于是整块区域等于没有可拖的地方。这里直接调系统的原生拖动。
 */
async function startDrag(e: PointerEvent) {
  if (e.button !== 0) return;
  // 输入框、按钮这些要正常交互，不能拿来拖窗口
  if ((e.target as HTMLElement).closest("input, button, select, textarea, a")) return;
  await getCurrentWindow().startDragging();
}

function setTab(t: typeof tab.value) {
  tab.value = t;
}

function onKey(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (editorOpen.value) {
    editorOpen.value = false;
    return;
  }
  void api.hidePanel();
}

// 面板的收起交给 Rust 侧处理：失焦时判断焦点是否落在本进程自己的窗口上，
// 是（用户去点小部件）就保持不动，否才收起。前端不再自己判断，避免两边打架。

onMounted(async () => {
  window.addEventListener("keydown", onKey);

  // Rust 侧唤出面板时会带上意图：直接看板 / 新建 / 设置
  await listen("panel:show", () => {
    quickFocus.value++;
  });
  await listen("panel:add", () => {
    tab.value = "today";
    quickFocus.value++;
  });
  await listen("panel:settings", () => {
    tab.value = "settings";
  });
  // 小部件里点了某个任务 → 直接打开它的编辑窗
  await listen("panel:edit", (e) => {
    const task = state.tasks.find((t) => t.id === (e.payload as number));
    if (task) openEditor(task);
  });

  quickFocus.value++;
});

onUnmounted(() => window.removeEventListener("keydown", onKey));

const shiftWeek = (n: number) => {
  anchor.value = addDays(anchor.value, n * 7);
};
const weekIsCurrent = computed(
  () => dayKey(anchor.value) === dayKey(state.now) || tab.value !== "week",
);
</script>

<template>
  <div class="win panel">
    <!-- 抓手：给面板一个明确的可拖区域，按空白处也能拖整条标签栏 -->
    <div class="p-grip" title="拖动面板" @pointerdown="startDrag"><i /></div>

    <div class="p-top">
      <QuickAdd :focus-signal="quickFocus" @submit="onQuick" />
    </div>

    <nav class="tabs" @pointerdown="startDrag">
      <button data-tab="today" :class="{ on: tab === 'today' }" @click="setTab('today')">今天</button>
      <button data-tab="week" :class="{ on: tab === 'week' }" @click="setTab('week')">周</button>
      <button data-tab="month" :class="{ on: tab === 'month' }" @click="setTab('month')">月</button>
      <span class="t-sp" />
      <template v-if="tab === 'week'">
        <button class="ghost" title="上一周" @click="shiftWeek(-1)">‹</button>
        <button
          class="ghost today-btn"
          :class="{ dim: !weekIsCurrent }"
          title="回到本周"
          @click="anchor = new Date(state.now)"
        >
          •
        </button>
        <button class="ghost" title="下一周" @click="shiftWeek(1)">›</button>
      </template>
      <button
        class="ghost"
        :class="{ active: tab === 'settings' }"
        title="设置"
        @click="tab = tab === 'settings' ? 'today' : 'settings'"
      >
        ⚙
      </button>
    </nav>

    <div class="p-body">
      <TodayAxis v-if="tab === 'today'" @edit="openEditor" />
      <WeekGrid v-else-if="tab === 'week'" :anchor="anchor" @edit="openEditor" />
      <MonthView v-else-if="tab === 'month'" :anchor="anchor" @edit="openEditor" />
      <SettingsView v-else />
    </div>

    <footer class="p-foot">
      <span>{{ footLeft }}</span>
      <span class="p-hint">双击 Ctrl 唤出 · Esc 收起</span>
    </footer>

    <!-- 保存后 store 会收到 Rust 广播自动刷新，这里只需收起小窗 -->
    <TaskEditor
      :open="editorOpen"
      :task="editing"
      :prefill="prefill"
      @close="editorOpen = false"
    />
  </div>
</template>

<style scoped>
.panel {
  padding: 0 14px 0;
}

/* 顶部抓手：明确告诉用户「这里可以拖」 */
.p-grip {
  height: 14px;
  flex: none;
  margin: 0 -14px;
  display: grid;
  place-items: center;
  cursor: grab;
}

.p-grip:active {
  cursor: grabbing;
}

.p-grip i {
  width: 34px;
  height: 3px;
  border-radius: 2px;
  background: var(--hairline);
  transition: background 0.15s;
}

.p-grip:hover i {
  background: var(--text-3);
}

.p-top {
  padding: 3px 0 2px;
}

.tabs {
  display: flex;
  align-items: center;
  gap: 2px;
  border-bottom: 1px solid var(--hairline);
  padding: 0 2px;
}

.tabs button[data-tab] {
  padding: 7px 13px;
  font-size: 12.5px;
  color: var(--text-2);
  border-radius: 8px 8px 0 0;
  position: relative;
  transition: color 0.15s;
}

.tabs button[data-tab].on {
  color: var(--text-1);
  font-weight: 600;
}

.tabs button[data-tab].on::after {
  content: "";
  position: absolute;
  left: 13px;
  right: 13px;
  bottom: -1px;
  height: 2px;
  border-radius: 2px;
  background: var(--accent);
}

.t-sp {
  flex: 1;
}

.ghost.active {
  background: var(--accent-weak);
  color: var(--accent-deep);
}

[data-mode="dark"] .ghost.active {
  color: var(--accent);
}

.today-btn {
  font-size: 16px;
}

.today-btn.dim {
  color: var(--accent);
}

.p-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  margin: 0 -6px;
  padding: 0 6px;
}

.p-foot {
  border-top: 1px solid var(--hairline);
  padding: 8px 0 9px;
  font-size: 11px;
  color: var(--text-3);
  display: flex;
  justify-content: space-between;
  margin: 0 -14px;
  padding-left: 18px;
  padding-right: 18px;
}

.p-hint {
  color: var(--text-3);
}
</style>
