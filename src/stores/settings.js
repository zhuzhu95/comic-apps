import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";

export const useSettingsStore = defineStore("settings", {
  state: () => ({
    autoUpdate: true,
    libraryPath: null,
    appVersion: "",
    loaded: false,
  }),
  actions: {
    async load() {
      try {
        const settings = await invoke("get_app_settings");
        this.autoUpdate = settings.auto_update;
        this.libraryPath = settings.library;
        this.appVersion = await getVersion();
      } catch (e) {
        console.error("加载设置失败:", e);
      } finally {
        this.loaded = true;
      }
    },
    async setAutoUpdate(enabled) {
      this.autoUpdate = enabled;
      try {
        await invoke("set_auto_update", { enabled });
      } catch (e) {
        this.autoUpdate = !enabled;
        console.error("保存设置失败:", e);
      }
    },
  },
});
