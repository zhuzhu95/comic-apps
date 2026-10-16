import { reactive } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask, message } from "@tauri-apps/plugin-dialog";

// 供设置页显示的检查/下载状态
export const updateState = reactive({
  checking: false,
  downloading: false,
  downloaded: 0,
  total: 0,
});

/**
 * 检查更新。
 * manual 为 true 时给出完整提示（含「已是最新」与错误），否则静默。
 */
export async function checkForUpdate({ manual = false } = {}) {
  if (updateState.checking || updateState.downloading) return;
  updateState.checking = true;
  try {
    const update = await check();
    if (!update) {
      if (manual) await message("当前已是最新版本。", { title: "检查更新" });
      return;
    }
    const yes = await ask(
      `发现新版本 ${update.version}（当前 ${update.currentVersion}）\n\n${update.body || ""}\n\n是否现在下载并安装？`,
      { title: "发现新版本", okLabel: "更新", cancelLabel: "稍后" }
    );
    if (!yes) return;

    updateState.downloading = true;
    updateState.downloaded = 0;
    updateState.total = 0;
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        updateState.total = event.data.contentLength || 0;
      } else if (event.event === "Progress") {
        updateState.downloaded += event.data.chunkLength;
      }
    });
    updateState.downloading = false;

    const restart = await ask("更新已下载完成，重启应用后生效。", {
      title: "更新完成",
      okLabel: "立即重启",
      cancelLabel: "稍后",
    });
    if (restart) await relaunch();
  } catch (e) {
    if (manual) {
      await message(`检查更新失败：${e}`, { title: "检查更新", kind: "error" });
    }
  } finally {
    updateState.checking = false;
    updateState.downloading = false;
  }
}
