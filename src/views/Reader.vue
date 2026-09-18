<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";

const route = useRoute();
const router = useRouter();
const comicId = route.params.id;

const loading = ref(true);
const error = ref("");
const pages = ref([]); // 页面 URL 列表
const pageCount = ref(0);
const currentPage = ref(0); // 0 起始
const mode = ref("paged"); // paged | scroll
const direction = ref(localStorage.getItem("readDirection") || "ltr"); // ltr | rtl
const showToolbar = ref(true);

function pageUrl(index) {
  return convertFileSrc(pages.value[index], "img");
}

// ---------- 初始化 ----------
onMounted(async () => {
  try {
    const paths = await invoke("get_comic_pages", { id: comicId });
    pages.value = paths;
    pageCount.value = paths.length;
    const progress = await invoke("get_progress", { id: comicId });
    if (progress) {
      currentPage.value = Math.min(progress.page, pageCount.value - 1);
      mode.value = progress.mode === "scroll" ? "scroll" : "paged";
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
  window.addEventListener("keydown", onKey);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  flushProgress();
});

// ---------- 进度保存（节流） ----------
let saveTimer = null;
watch([currentPage, mode], () => scheduleSave());

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(flushProgress, 400);
}

function flushProgress() {
  clearTimeout(saveTimer);
  if (!pageCount.value) return;
  invoke("save_progress", {
    id: comicId,
    page: currentPage.value,
    mode: mode.value,
  }).catch(() => {});
}

// ---------- 翻页模式 ----------
function nextPage() {
  if (currentPage.value < pageCount.value - 1) currentPage.value++;
}

function prevPage() {
  if (currentPage.value > 0) currentPage.value--;
}

// 日漫（rtl）时「下一页」在左边
function goForward() {
  direction.value === "rtl" ? prevPage() : nextPage();
}

function goBackward() {
  direction.value === "rtl" ? nextPage() : prevPage();
}

function onKey(e) {
  if (mode.value !== "paged") {
    return;
  }
  if (e.key === "ArrowRight") goForward();
  else if (e.key === "ArrowLeft") goBackward();
  else if (e.key === "Escape") back();
}

function onPageClick(e) {
  if (mode.value !== "paged") return;
  const x = e.clientX / window.innerWidth;
  if (x < 0.3) goBackward();
  else if (x > 0.7) goForward();
  else showToolbar.value = !showToolbar.value;
}

// ---------- 条漫模式：跟踪当前页 ----------
let observer = null;

function onScrollContainer(el) {
  if (!el || mode.value !== "scroll") return;
  observer?.disconnect();
  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const idx = Number(entry.target.dataset.index);
          if (!Number.isNaN(idx)) currentPage.value = idx;
          break;
        }
      }
    },
    { root: el, threshold: 0.1 }
  );
  el.querySelectorAll("[data-index]").forEach((img) => observer.observe(img));
  // 跳转到上次进度
  const target = el.querySelector(`[data-index="${currentPage.value}"]`);
  target?.scrollIntoView();
}

watch(mode, async (m) => {
  if (m !== "scroll") {
    observer?.disconnect();
    observer = null;
    return;
  }
  await new Promise((r) => setTimeout(r, 50));
  const el = document.querySelector(".scroll-container");
  if (el) onScrollContainer(el);
});

function toggleDirection() {
  direction.value = direction.value === "ltr" ? "rtl" : "ltr";
  localStorage.setItem("readDirection", direction.value);
}

function back() {
  flushProgress();
  router.push("/");
}
</script>

<template>
  <div class="reader" @click="onPageClick">
    <div v-if="loading" class="center hint">加载中…</div>
    <div v-else-if="error" class="center error">{{ error }} <button @click.stop="back()">返回</button></div>

    <template v-else>
      <!-- 工具条 -->
      <div v-if="showToolbar" class="toolbar" @click.stop>
        <button @click="back()">← 返回</button>
        <span class="page-info">{{ currentPage + 1 }} / {{ pageCount }}</span>
        <div class="spacer" />
        <button v-if="mode === 'paged'" @click="toggleDirection()">
          {{ direction === "rtl" ? "日漫模式（右→左）" : "普通模式（左→右）" }}
        </button>
        <button @click="mode = mode === 'paged' ? 'scroll' : 'paged'">
          {{ mode === "paged" ? "切换条漫" : "切换翻页" }}
        </button>
      </div>

      <!-- 翻页模式 -->
      <div v-if="mode === 'paged'" class="paged">
        <img :src="pageUrl(currentPage)" :key="currentPage" alt="page" draggable="false" />
      </div>

      <!-- 条漫模式 -->
      <div v-else class="scroll-container" :ref="onScrollContainer">
        <img
          v-for="(p, i) in pages"
          :key="p"
          :data-index="i"
          :src="convertFileSrc(p, 'img')"
          loading="lazy"
          alt="page"
          draggable="false"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.reader {
  height: 100%;
  background: #101013;
  position: relative;
  user-select: none;
}

.center {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
  justify-content: center;
}

.hint {
  color: #888;
}

.error {
  color: #ff7a7a;
}

.toolbar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: rgba(20, 20, 24, 0.85);
  backdrop-filter: blur(4px);
}

.spacer {
  flex: 1;
}

.page-info {
  color: #aaa;
  font-variant-numeric: tabular-nums;
}

.paged {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.paged img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.scroll-container {
  height: 100%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.scroll-container img {
  width: 100%;
  max-width: 900px;
  display: block;
}
</style>
