<script setup>
import { onMounted } from "vue";
import { useSettingsStore } from "./stores/settings";
import { checkForUpdate } from "./updater";

const settings = useSettingsStore();

onMounted(async () => {
  await settings.load();
  if (settings.autoUpdate) checkForUpdate({ manual: false });
});
</script>

<template>
  <router-view />
</template>
