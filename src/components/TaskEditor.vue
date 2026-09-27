<script setup lang="ts">
// 新建 / 编辑任务小窗。回车保存、Esc 取消，字段都在键盘可达范围内。
import { computed, nextTick, ref, watch } from "vue";
import { api, CATEGORIES, type Category, type Task, type TaskPrefill } from "../lib/api";
import { state } from "../lib/store";
import { dayKey, fromStamp, pad } from "../lib/time";

const props = defineProps<{
  open: boolean;
  task?: Task | null;
  prefill?: TaskPrefill | null;
}>();
const emit = defineEmits<{ (e: "close"): void; (e: "saved", task: Task): void }>();

const title = ref("");
const date = ref("");
const time = ref("09:00");
const category = ref<Category>("errand");
const priority = ref(0);
const notes = ref("");
const remind = ref("10");
const titleEl = ref<HTMLInputElement | null>(null);

const isEdit = computed(() => !!props.task?.id);

const REMIND_OPTIONS = [
  { v: "10", label: "提前 10 分钟" },
  { v: "0", label: "准点提醒" },
  { v: "30", label: "提前 30 分钟" },
  { v: "60", label: "提前 1 小时" },
  { v: "1440", label: "提前 1 天" },
  { v: "none", label: "不提醒" },
];

/** 打开时按「已有任务 → 快速添加解析结果 → 默认值」的优先级填表 */
watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    const t = props.task;
    const p = props.prefill;

    if (t?.id) {
      title.value = t.title;
      const due = fromStamp(t.dueAt);
      date.value = due ? dayKey(due) : "";
      time.value = due ? `${pad(due.getHours())}:${pad(due.getMinutes())}` : "09:00";
      category.value = t.category;
      priority.value = t.priority;
      notes.value = t.notes;
      const first = t.remindOffsets?.[0];
      remind.value =
        !t.remindOffsets || t.remindOffsets.length === 0
          ? "none"
          : String(first ?? 10);
    } else {
      const due = fromStamp(p?.dueAt ?? null) ?? defaultDue();
      title.value = p?.title ?? "";
      date.value = dayKey(due);
      time.value = `${pad(due.getHours())}:${pad(due.getMinutes())}`;
      category.value = p?.category ?? "errand";
      priority.value = p?.priority ?? 0;
      notes.value = "";
      remind.value = String(state.settings?.defaultRemindOffsets?.[0] ?? 10);
    }
    await nextTick();
    titleEl.value?.focus();
    titleEl.value?.select();
  },
);

/** 默认截止：下一个整点 */
function defaultDue(): Date {
  const d = new Date(state.now.getTime() + 3600_000);
  d.setMinutes(0, 0, 0);
  return d;
}

function reminderOffsets(): number[] {
  if (remind.value === "none") return [];
  const n = Number(remind.value);
  return n === 0 ? [0] : [n, 0];
}

async function submit() {
  const text = title.value.trim();
  if (!text) {
    titleEl.value?.focus();
    return;
  }
  const dueAt = date.value ? `${date.value}T${time.value || "09:00"}:00` : null;
  const saved = await api.saveTask({
    id: props.task?.id ?? null,
    title: text,
    notes: notes.value,
    dueAt,
    category: category.value,
    priority: priority.value,
    remindOffsets: reminderOffsets(),
    repeatRule: props.task?.repeatRule ?? props.prefill?.repeatRule ?? null,
  });
  emit("saved", saved);
  emit("close");
}

async function remove() {
  if (!props.task?.id) return;
  await api.deleteTask(props.task.id);
  emit("close");
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey && (e.target as HTMLElement)?.tagName !== "TEXTAREA") {
    e.preventDefault();
    void submit();
  }
}
</script>

<template>
  <div v-if="open" class="mask" @click.self="emit('close')" @keydown.esc="emit('close')">
    <div class="pop" role="dialog" @keydown="onKey">
      <div class="f-head">
        <b>{{ isEdit ? "编辑任务" : "新建任务" }}</b>
        <button class="ghost" title="关闭" @click="emit('close')">×</button>
      </div>

      <div class="frow">
        <label class="f-label">任务</label>
        <input ref="titleEl" v-model="title" class="f-in" placeholder="要做什么？" />
      </div>

      <div class="frow">
        <label class="f-label">截止</label>
        <input v-model="date" type="date" class="f-in" />
        <input v-model="time" type="time" class="f-in time" />
        <button
          v-if="priority === 0"
          class="mini-btn"
          title="标记为重要"
          @click="priority = 1"
        >
          ! 重要
        </button>
        <button v-else class="mini-btn ok" title="取消重要" @click="priority = 0">
          ! 重要
        </button>
      </div>

      <div class="frow">
        <label class="f-label">标签</label>
        <div class="fdots">
          <button
            v-for="c in CATEGORIES"
            :key="c.key"
            class="fdot"
            :class="{ on: category === c.key }"
            :style="{ '--tc': c.color }"
            @click="category = c.key"
          >
            <i />{{ c.label }}
          </button>
        </div>
      </div>

      <div class="frow">
        <label class="f-label">提醒</label>
        <select v-model="remind" class="f-in">
          <option v-for="o in REMIND_OPTIONS" :key="o.v" :value="o.v">{{ o.label }}</option>
        </select>
      </div>

      <div class="frow">
        <label class="f-label">备注</label>
        <textarea v-model="notes" class="f-in ta" rows="2" placeholder="可选" />
      </div>

      <div class="f-foot">
        <span class="f-hint">回车保存 · Esc 取消</span>
        <span class="f-actions">
          <button v-if="isEdit" class="btn danger" @click="remove">删除</button>
          <button class="btn gho" @click="emit('close')">取消</button>
          <button class="btn pri" @click="submit">{{ isEdit ? "保存" : "＋ 添加" }}</button>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: absolute;
  inset: 0;
  z-index: 20;
  border-radius: var(--r-lg);
  background: rgba(15, 18, 22, 0.3);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 64px;
}

.pop {
  width: calc(100% - 32px);
  background: var(--solid);
  border: 1px solid var(--hairline);
  border-radius: var(--r-md);
  box-shadow: 0 18px 44px rgba(10, 14, 20, 0.26);
  padding: 12px 14px 11px;
  animation: popin 0.18s cubic-bezier(0.22, 0.9, 0.36, 1) both;
}

@keyframes popin {
  from {
    opacity: 0;
    transform: translateY(6px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.f-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 9px;
}

.f-head b {
  font-size: 12.5px;
}

.frow {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.frow:has(.ta) {
  align-items: flex-start;
}

.f-label {
  font-size: 11.5px;
  color: var(--text-2);
  width: 30px;
  flex: none;
}

.f-in {
  flex: 1;
  min-width: 0;
  background: var(--surface-2);
  border: 1px solid var(--hairline);
  border-radius: 8px;
  padding: 6px 9px;
  font-size: 12.5px;
  color: var(--text-1);
  outline: none;
  transition: border-color 0.15s;
}

.f-in:focus {
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
}

.f-in.time {
  flex: 0 0 104px;
}

.f-in.ta {
  resize: none;
  font-family: inherit;
  line-height: 1.5;
}

.fdots {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.fdot {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  padding: 2.5px 8px;
  border-radius: 14px;
  border: 1px solid transparent;
  color: var(--text-2);
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.fdot i {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--tc);
}

.fdot.on {
  background: color-mix(in srgb, var(--tc) 15%, transparent);
  border-color: color-mix(in srgb, var(--tc) 40%, transparent);
  color: color-mix(in srgb, var(--tc) 70%, var(--text-1) 30%);
}

.f-foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 9px;
}

.f-hint {
  font-size: 10.5px;
  color: var(--text-3);
}

.f-actions {
  display: flex;
  gap: 8px;
}
</style>
