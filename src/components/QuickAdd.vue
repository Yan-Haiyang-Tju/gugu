<script setup lang="ts">
// 快速添加：输入一句话，实时把解析结果以胶囊回显，回车后进入编辑小窗确认。
import { computed, nextTick, ref, watch } from "vue";
import { categoryOf, type TaskPrefill } from "../lib/api";
import { parseQuick } from "../lib/nlp";

const props = defineProps<{ focusSignal?: number }>();
const emit = defineEmits<{ (e: "submit", prefill: TaskPrefill): void }>();

const text = ref("");
const inputEl = ref<HTMLInputElement | null>(null);

const parsed = computed(() => (text.value.trim() ? parseQuick(text.value) : null));

watch(
  () => props.focusSignal,
  async () => {
    await nextTick();
    inputEl.value?.focus();
  },
);

function submit() {
  const p = parsed.value;
  // 至少要有个标题或时间，否则忽略这次回车
  if (!p || (!p.title && !p.dueAt)) return;
  emit("submit", {
    title: p.title,
    dueAt: p.dueAt,
    category: p.category,
    priority: p.priority,
    repeatRule: p.repeatRule,
  });
  text.value = "";
}
</script>

<template>
  <div class="quick-wrap">
    <div class="quick">
      <span class="q-plus" title="新建任务" @click="submit">+</span>
      <input
        ref="inputEl"
        v-model="text"
        placeholder="添加任务：明天15:00 交周报 #工作 !重要"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter="submit"
      />
      <span class="enter-hint">回车</span>
    </div>

    <div v-if="parsed" class="chips">
      <span v-for="(tk, i) in parsed.tokens" :key="i" class="chip" :class="tk.kind">
        {{ tk.label }}
      </span>
      <span
        v-if="parsed.category"
        class="chip tag"
        :style="{ '--tc': categoryOf(parsed.category).color }"
      >
        {{ categoryOf(parsed.category).label }}
      </span>
      <span v-if="parsed.title" class="chip ttl">{{ parsed.title }}</span>
    </div>
  </div>
</template>

<style scoped>
.quick-wrap {
  padding-bottom: 8px;
}

.quick {
  display: flex;
  align-items: center;
  gap: 9px;
  background: var(--surface-2);
  border: 1px solid var(--hairline);
  border-radius: var(--r-md);
  padding: 9px 12px;
  transition: border-color 0.15s;
}

.quick:focus-within {
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
}

.q-plus {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--on-accent);
  display: grid;
  place-items: center;
  font-size: 14px;
  line-height: 1;
  flex: none;
  font-weight: 500;
  cursor: pointer;
}

.quick input {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  font-size: 13px;
  color: var(--text-1);
}

.quick input::placeholder {
  color: var(--text-3);
}

.enter-hint {
  font-size: 10.5px;
  color: var(--text-3);
  border: 1px solid var(--hairline);
  border-radius: 5px;
  padding: 1.5px 6px;
  flex: none;
}

.chips {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  min-height: 21px;
  flex-wrap: wrap;
  align-items: center;
}

.chip {
  font-size: 11px;
  padding: 2px 9px;
  border-radius: 20px;
  background: var(--accent-weak);
  color: color-mix(in srgb, var(--accent) 72%, var(--text-1) 28%);
}

.chip.tag {
  background: color-mix(in srgb, var(--tc) 15%, transparent);
  color: color-mix(in srgb, var(--tc) 75%, var(--text-1) 25%);
}

.chip.priority {
  background: color-mix(in srgb, var(--danger) 13%, transparent);
  color: var(--danger);
}

.chip.repeat {
  background: color-mix(in srgb, var(--c-study) 15%, transparent);
  color: color-mix(in srgb, var(--c-study) 75%, var(--text-1) 25%);
}

.chip.ttl {
  background: var(--surface-2);
  color: var(--text-2);
}
</style>
