<script setup>
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "../stores/settings";
import { useLibraryStore } from "../stores/library";
import { checkForUpdate, updateState } from "../updater";

const router = useRouter();
const settings = useSettingsStore();
const library = useLibraryStore();

onMounted(() => {
  if (!settings.loaded) settings.load();
});

const progressText = computed(() => {
  if (updateState.checking) return "正在检查更新…";
  if (updateState.downloading) {
    if (!updateState.total) return "正在下载更新…";
    const pct = Math.round((updateState.downloaded / updateState.total) * 100);
    const mb = (updateState.downloaded / 1024 / 1024).toFixed(1);
    const totalMb = (updateState.total / 1024 / 1024).toFixed(1);
    return `正在下载更新… ${pct}%（${mb} / ${totalMb} MB）`;
  }
  return "";
});

async function onToggleAutoUpdate(event) {
  await settings.setAutoUpdate(event.target.checked);
}

async function onChooseLibrary() {
  await library.chooseLibrary();
  if (library.libraryPath) settings.libraryPath = library.libraryPath;
}
</script>

<template>
  <div class="settings">
    <header class="toolbar">
      <button @click="router.push('/')">← 返回</button>
      <h1>设置</h1>
    </header>

    <div class="body">
      <section class="group">
        <h2>更新</h2>
        <label class="row">
          <input
            type="checkbox"
            :checked="settings.autoUpdate"
            @change="onToggleAutoUpdate"
          />
          <span>启动时自动检查更新</span>
        </label>
        <div class="row">
          <span class="label">当前版本</span>
          <span>{{ settings.appVersion || "…" }}</span>
        </div>
        <div class="row">
          <button
            :disabled="updateState.checking || updateState.downloading"
            @click="checkForUpdate({ manual: true })"
          >
            检查更新
          </button>
          <span v-if="progressText" class="progress">{{ progressText }}</span>
        </div>
      </section>

      <section class="group">
        <h2>资源库</h2>
        <div class="row">
          <span class="path">{{ settings.libraryPath || "未设置" }}</span>
          <button @click="onChooseLibrary">更换书库</button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-bottom: 1px solid #2c2c34;
}

.toolbar h1 {
  font-size: 1.1em;
  margin: 0;
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 22px;
  max-width: 640px;
}

.group {
  margin-bottom: 28px;
}

.group h2 {
  font-size: 0.95em;
  color: #999;
  font-weight: normal;
  margin: 0 0 10px;
  border-bottom: 1px solid #2c2c34;
  padding-bottom: 6px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
}

.row .label {
  color: #999;
}

.row input[type="checkbox"] {
  width: 16px;
  height: 16px;
  accent-color: #6a6a8a;
}

.progress {
  color: #888;
  font-size: 0.9em;
}

.path {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #bbb;
}
</style>
