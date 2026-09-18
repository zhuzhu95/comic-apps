<script setup>
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ask } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useLibraryStore } from "../stores/library";

const store = useLibraryStore();
const router = useRouter();
const searchText = ref("");
let searchTimer = null;

onMounted(() => store.init());

function onSearch() {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => store.search(searchText.value.trim()), 200);
}

function coverUrl(comic) {
  return comic.cover ? convertFileSrc(comic.cover, "img") : null;
}

function progressText(comic) {
  if (comic.progress_page == null || !comic.page_count) return "";
  const pct = Math.round(((comic.progress_page + 1) / comic.page_count) * 100);
  return pct >= 100 ? "已读完" : `${pct}%`;
}

async function onDelete(comic) {
  const confirmed = await ask(`确定将《${comic.name}》从书库移除？`, {
    title: "删除漫画",
    kind: "warning",
  });
  if (!confirmed) return;
  const removeFiles = await ask("是否同时删除磁盘上的源文件？", {
    title: "删除漫画",
    kind: "warning",
    okLabel: "删除文件",
    cancelLabel: "仅移除记录",
  });
  await store.removeComic(comic.id, removeFiles);
}

function openReader(comic) {
  router.push(`/reader/${comic.id}`);
}
</script>

<template>
  <div class="library">
    <!-- 首次启动：选择资源库 -->
    <div v-if="!store.libraryPath && !store.loading" class="welcome">
      <h1>漫画阅读器</h1>
      <p>选择一个文件夹作为漫画资源库。<br />该文件夹下每个子文件夹 / zip / pdf 都会被识别为一部漫画。</p>
      <button class="primary" @click="store.chooseLibrary()">选择资源库文件夹</button>
      <p v-if="store.error" class="error">{{ store.error }}</p>
    </div>

    <template v-else>
      <header class="toolbar">
        <input
          v-model="searchText"
          class="search"
          placeholder="搜索漫画名称…"
          @input="onSearch"
        />
        <div class="spacer" />
        <button title="导入 zip / pdf 文件" @click="store.importFiles()">导入文件</button>
        <button title="导入内含图片或 zip 的文件夹" @click="store.importFolder()">导入文件夹</button>
        <button title="重新扫描资源库" @click="store.scan()">扫描</button>
        <button title="更换资源库文件夹" @click="store.chooseLibrary()">更换书库</button>
        <div class="view-toggle">
          <button
            :class="{ active: store.viewMode === 'grid' }"
            title="平铺视图"
            @click="store.setViewMode('grid')"
          >
            平铺
          </button>
          <button
            :class="{ active: store.viewMode === 'list' }"
            title="列表视图"
            @click="store.setViewMode('list')"
          >
            列表
          </button>
        </div>
      </header>

      <p v-if="store.error" class="error">{{ store.error }}</p>
      <div v-if="store.loading" class="hint">加载中…</div>

      <!-- 平铺视图：只显示封面 -->
      <div v-else-if="store.viewMode === 'grid'" class="grid">
        <div
          v-for="comic in store.comics"
          :key="comic.id"
          class="card"
          :title="comic.name"
          @click="openReader(comic)"
        >
          <img v-if="coverUrl(comic)" :src="coverUrl(comic)" :alt="comic.name" loading="lazy" />
          <div v-else class="no-cover">无封面</div>
          <span v-if="progressText(comic)" class="badge">{{ progressText(comic) }}</span>
          <button class="delete" title="删除" @click.stop="onDelete(comic)">×</button>
        </div>
        <div v-if="!store.comics.length" class="hint">没有找到漫画，点击「导入」或「扫描」</div>
      </div>

      <!-- 列表视图：只显示名称 -->
      <div v-else class="list">
        <div
          v-for="comic in store.comics"
          :key="comic.id"
          class="row"
          @click="openReader(comic)"
        >
          <span class="name">{{ comic.name }}</span>
          <span class="meta">{{ progressText(comic) }}</span>
          <button class="delete" title="删除" @click.stop="onDelete(comic)">×</button>
        </div>
        <div v-if="!store.comics.length" class="hint">没有找到漫画，点击「导入」或「扫描」</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.library {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.welcome {
  margin: auto;
  text-align: center;
  max-width: 420px;
  line-height: 1.8;
}

.primary {
  font-size: 1.05em;
  padding: 0.6em 1.6em;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid #2c2c34;
  flex-wrap: wrap;
}

.search {
  width: 260px;
}

.spacer {
  flex: 1;
}

.view-toggle {
  display: flex;
  gap: 0;
}

.view-toggle button.active {
  background-color: #4a4a5c;
  border-color: #6a6a8a;
}

.error {
  color: #ff7a7a;
  padding: 0 14px;
}

.hint {
  color: #888;
  padding: 30px;
  text-align: center;
  width: 100%;
}

.grid {
  flex: 1;
  overflow-y: auto;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
  padding: 14px;
  align-content: start;
}

.card {
  position: relative;
  aspect-ratio: 3 / 4;
  border-radius: 8px;
  overflow: hidden;
  background: #26262d;
  cursor: pointer;
  border: 1px solid #2e2e38;
}

.card:hover {
  border-color: #6a6a8a;
}

.card img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.no-cover {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
}

.badge {
  position: absolute;
  left: 6px;
  bottom: 6px;
  background: rgba(0, 0, 0, 0.65);
  border-radius: 4px;
  padding: 1px 6px;
  font-size: 12px;
}

.delete {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 24px;
  height: 24px;
  padding: 0;
  line-height: 1;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  opacity: 0;
}

.card:hover .delete,
.row:hover .delete {
  opacity: 1;
}

.list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 14px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 8px;
  border-bottom: 1px solid #26262d;
  cursor: pointer;
  position: relative;
}

.row:hover {
  background: #23232a;
}

.row .name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row .meta {
  color: #888;
  font-size: 0.9em;
}

.row .delete {
  position: static;
  opacity: 0;
}
</style>
