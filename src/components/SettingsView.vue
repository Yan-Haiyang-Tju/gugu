<script setup lang="ts">
// 设置页：外观可调（主题 / 深浅 / 透明度 / 毛玻璃）是这套主题系统的入口。
import { computed, onMounted, ref } from "vue";
import { api, type ThemeName } from "../lib/api";
import { patchSettings, state } from "../lib/store";

const s = computed(() => state.settings);
const autostart = ref(false);
const dataPath = ref("");
const exportPath = ref<string | null>(null);
const busy = ref(false);

const THEMES: { key: ThemeName; label: string; color: string }[] = [
  { key: "celadon", label: "青瓷绿", color: "#3E9B7A" },
  { key: "azure", label: "天青蓝", color: "#4A90D9" },
  { key: "tangerine", label: "蜜柑橘", color: "#E8956D" },
];

onMounted(async () => {
  autostart.value = await api.getAutostart();
  dataPath.value = await api.dataDir();
});

async function toggleAutostart(on: boolean) {
  autostart.value = await api.setAutostart(on);
}

async function doExport() {
  busy.value = true;
  try {
    exportPath.value = await api.exportData();
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div v-if="s" class="settings">
    <!-- 外观 -->
    <section>
      <h4>外观</h4>
      <div class="row">
        <label>主色</label>
        <div class="themes">
          <button
            v-for="t in THEMES"
            :key="t.key"
            class="theme"
            :class="{ on: s.theme === t.key }"
            @click="patchSettings({ theme: t.key })"
          >
            <i :style="{ background: t.color }" />{{ t.label }}
          </button>
        </div>
      </div>
      <div class="row">
        <label>深浅</label>
        <div class="seg">
          <button :class="{ on: s.mode === 'light' }" @click="patchSettings({ mode: 'light' })">
            浅色
          </button>
          <button :class="{ on: s.mode === 'dark' }" @click="patchSettings({ mode: 'dark' })">
            深色
          </button>
        </div>
      </div>
      <div class="row">
        <label>透明度</label>
        <input
          type="range"
          min="0.5"
          max="0.95"
          step="0.01"
          :value="s.glassAlpha"
          @input="patchSettings({ glassAlpha: Number(($event.target as HTMLInputElement).value) })"
        />
        <b class="val">{{ Math.round(s.glassAlpha * 100) }}%</b>
      </div>
      <div class="row">
        <label>毛玻璃</label>
        <input
          type="checkbox"
          :checked="s.blur"
          @change="patchSettings({ blur: ($event.target as HTMLInputElement).checked })"
        />
        <span class="tip">开启后小部件走系统磨砂（能透出模糊的桌面）；关掉是干净实心卡片，文字更清晰</span>
      </div>
    </section>

    <!-- 桌面 -->
    <section>
      <h4>桌面</h4>
      <div class="row">
        <label>显示小部件</label>
        <input
          type="checkbox"
          :checked="s.widgetVisible"
          @change="patchSettings({ widgetVisible: ($event.target as HTMLInputElement).checked })"
        />
      </div>
      <div class="row">
        <label>小部件层级</label>
        <div class="seg">
          <button
            :class="{ on: s.widgetLayer === 'float' }"
            @click="patchSettings({ widgetLayer: 'float' })"
          >
            浮在图标上
          </button>
          <button
            :class="{ on: s.widgetLayer === 'wallpaper' }"
            @click="patchSettings({ widgetLayer: 'wallpaper' })"
          >
            沉到图标下
          </button>
        </div>
      </div>
      <p class="tip-line">
        浮在图标上：随时能点、能拖，代价是盖住它下面那块桌面图标；
        沉到图标下：完全不占地方，但收不到鼠标点击，只能双击 Ctrl 唤出面板操作。
      </p>
      <div class="row">
        <label>鼠标穿透</label>
        <input
          type="checkbox"
          :checked="s.clickThrough"
          @change="patchSettings({ clickThrough: ($event.target as HTMLInputElement).checked })"
        />
        <span class="tip">开启后小部件不再响应点击，只能双击 Ctrl 唤出面板</span>
      </div>
      <div class="row">
        <label>位置</label>
        <button class="btn gho" @click="api.resetWidgetPos()">重置到屏幕右上角</button>
      </div>
    </section>

    <!-- 快捷键 -->
    <section>
      <h4>快捷键</h4>
      <div class="row">
        <label>双击 Ctrl</label>
        <input
          type="checkbox"
          :checked="s.hotkeyEnabled"
          @change="patchSettings({ hotkeyEnabled: ($event.target as HTMLInputElement).checked })"
        />
        <span class="tip">任意界面连按两次 Ctrl 唤出/收起面板</span>
      </div>
      <div class="row">
        <label>连按间隔</label>
        <input
          type="range"
          min="200"
          max="900"
          step="20"
          :value="s.hotkeyWindowMs"
          @input="patchSettings({ hotkeyWindowMs: Number(($event.target as HTMLInputElement).value) })"
        />
        <b class="val">{{ s.hotkeyWindowMs }}ms</b>
      </div>
    </section>

    <!-- 提醒 -->
    <section>
      <h4>提醒</h4>
      <div class="row">
        <label>免打扰</label>
        <input
          type="checkbox"
          :checked="s.dnd"
          @change="patchSettings({ dnd: ($event.target as HTMLInputElement).checked })"
        />
        <span class="tip">暂停所有到点提醒（任务照常记录）</span>
      </div>
      <div class="row">
        <label>通知</label>
        <button class="btn gho" @click="api.testNotification()">发一条测试通知</button>
      </div>
    </section>

    <!-- 系统与数据 -->
    <section>
      <h4>系统</h4>
      <div class="row">
        <label>开机自启</label>
        <input
          type="checkbox"
          :checked="autostart"
          @change="toggleAutostart(($event.target as HTMLInputElement).checked)"
        />
      </div>
      <div class="row">
        <label>数据</label>
        <button class="btn gho" :disabled="busy" @click="doExport">导出 JSON 备份</button>
        <button class="btn gho" @click="api.openPath(dataPath)">打开数据目录</button>
      </div>
      <p v-if="exportPath" class="path">已导出：{{ exportPath }}</p>
      <p v-else-if="dataPath" class="path">数据位置：{{ dataPath }}</p>
    </section>

    <section class="about">
      <h4>关于</h4>
      <p>咕咕 GUGU · 常驻桌面的日历任务小部件</p>
      <p class="dim">Tauri 2 + Vue 3 · 数据全部保存在本机</p>
      <button class="btn danger" @click="api.quit()">退出咕咕</button>
    </section>
  </div>
</template>

<style scoped>
.settings {
  padding: 10px 4px 24px;
}

section {
  padding: 4px 6px 12px;
  border-bottom: 1px solid var(--hairline);
  margin-bottom: 10px;
}

section:last-child {
  border-bottom: none;
}

h4 {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--text-3);
  letter-spacing: 0.5px;
  padding: 6px 2px 8px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 2px;
  font-size: 12.5px;
  min-height: 28px;
}

.row > label {
  width: 74px;
  flex: none;
  color: var(--text-2);
  font-size: 12px;
}

.row input[type="checkbox"] {
  accent-color: var(--accent);
  width: 15px;
  height: 15px;
}

.row input[type="range"] {
  flex: 1;
  accent-color: var(--accent);
  max-width: 180px;
}

.val {
  font-size: 11.5px;
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
  min-width: 44px;
}

.tip {
  font-size: 10.5px;
  color: var(--text-3);
  line-height: 1.4;
  flex: 1;
}

.tip-line {
  font-size: 10.5px;
  color: var(--text-3);
  line-height: 1.5;
  padding: 0 2px 4px 84px;
}

.themes {
  display: flex;
  gap: 6px;
}

.theme {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  padding: 4px 10px;
  border-radius: 14px;
  border: 1px solid var(--hairline);
  color: var(--text-2);
  transition: border-color 0.15s, background 0.15s, color 0.15s;
}

.theme i {
  width: 9px;
  height: 9px;
  border-radius: 50%;
}

.theme.on {
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  background: var(--accent-weak);
  color: var(--text-1);
}

.seg {
  display: flex;
  background: var(--surface-2);
  border-radius: 8px;
  padding: 2px;
}

.seg button {
  padding: 3px 12px;
  border-radius: 6px;
  font-size: 11.5px;
  color: var(--text-2);
  transition: background 0.15s, color 0.15s;
}

.seg button.on {
  background: var(--accent);
  color: var(--on-accent);
}

.path {
  font-size: 10.5px;
  color: var(--text-3);
  padding: 2px;
  word-break: break-all;
  user-select: text;
}

.about p {
  font-size: 12px;
  color: var(--text-2);
  padding: 2px;
}

.about .dim {
  color: var(--text-3);
  font-size: 11px;
  padding-bottom: 10px;
}
</style>
