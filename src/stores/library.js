import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export const useLibraryStore = defineStore("library", {
  state: () => ({
    libraryPath: null,
    comics: [],
    viewMode: localStorage.getItem("viewMode") || "grid",
    loading: false,
    error: "",
  }),
  actions: {
    async init() {
      this.error = "";
      try {
        this.libraryPath = await invoke("get_library");
        if (this.libraryPath) await this.scan();
      } catch (e) {
        this.error = String(e);
      }
    },
    async chooseLibrary() {
      const dir = await open({ directory: true, title: "选择资源库文件夹" });
      if (!dir) return;
      this.loading = true;
      this.error = "";
      try {
        this.comics = await invoke("set_library", { path: dir });
        this.libraryPath = dir;
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async scan() {
      this.loading = true;
      this.error = "";
      try {
        this.comics = await invoke("scan_library");
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async search(query) {
      try {
        this.comics = await invoke("list_comics", { query });
      } catch (e) {
        this.error = String(e);
      }
    },
    setViewMode(mode) {
      this.viewMode = mode;
      localStorage.setItem("viewMode", mode);
    },
    async importFiles() {
      // 散图资源请用「导入文件夹」
      const paths = await open({
        multiple: true,
        title: "导入漫画文件（zip / pdf）",
        filters: [{ name: "漫画文件", extensions: ["zip", "pdf"] }],
      });
      if (!paths || paths.length === 0) return;
      this.loading = true;
      try {
        this.comics = await invoke("import_paths", { paths });
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async importFolder() {
      const path = await open({ directory: true, title: "导入漫画文件夹（内含图片或 zip）" });
      if (!path) return;
      this.loading = true;
      try {
        this.comics = await invoke("import_paths", { paths: [path] });
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },
    async removeComic(id, removeFiles) {
      try {
        this.comics = await invoke("delete_comic", { id, removeFiles });
      } catch (e) {
        this.error = String(e);
      }
    },
  },
});
