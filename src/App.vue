<script setup lang="ts">
// 双窗口共用一个入口，按 URL 参数分流（见 tauri.conf.json 的 ?window=widget|panel）
import { onMounted, ref } from "vue";
import PanelApp from "./components/PanelApp.vue";
import WidgetApp from "./components/WidgetApp.vue";
import { initStore, state } from "./lib/store";

const isPanel = new URLSearchParams(location.search).get("window") === "panel";
const bootError = ref<string | null>(null);

// 让 tokens.css 能按窗口类型给不同的底色（小部件钉成子窗口后系统模糊会失效，需要更实的底）
document.documentElement.dataset.win = isPanel ? "panel" : "widget";

onMounted(async () => {
  try {
    await initStore();
  } catch (e) {
    bootError.value = e instanceof Error ? e.message : String(e);
    console.error("[gugu] 初始化失败", e);
  }
});
</script>

<template>
  <div v-if="bootError" class="boot-error">
    <b>咕咕启动失败</b>
    <span>{{ bootError }}</span>
  </div>
  <template v-else-if="state.ready">
    <PanelApp v-if="isPanel" />
    <WidgetApp v-else />
  </template>
</template>

<style scoped>
.boot-error {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  text-align: center;
  border-radius: var(--r-lg);
  background: var(--surface);
  border: 1px solid var(--edge);
  color: var(--text-1);
  font-size: 12px;
}
.boot-error b {
  color: var(--danger);
}
.boot-error span {
  color: var(--text-2);
  user-select: text;
}
</style>
