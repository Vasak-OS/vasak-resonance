<script lang="ts" setup>
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { onMounted, Ref, ref } from 'vue';
import { toggleMainAndMiniPlayer } from '@/services/window.service';

const appWindow = getCurrentWindow();
const closeIcon: Ref<string> = ref('');
const minimizeIcon: Ref<string> = ref('');
const maximizeIcon: Ref<string> = ref('');
const miniIcon: Ref<string> = ref('');

onMounted(async () => {
  miniIcon.value = await getSymbolSource('screenshot-ui-window');
	closeIcon.value = await getSymbolSource('window-close');
	minimizeIcon.value = await getSymbolSource('window-minimize');
	maximizeIcon.value = await getSymbolSource('window-maximize');
});

const toggleMiniPlayer = async () => {
	await toggleMainAndMiniPlayer();
};
</script>
<template>
  <div class="flex gap-1" data-tauri-drag-region>
    <button
      type="button"
      class="px-2 py-1 bg-ui-bg/80 rounded-corner hover:bg-primary/20 border border-ui-border text-xs font-medium text-primary"
      title="Mini"
      aria-label="Mini"
      @click="toggleMiniPlayer"
    >
      <img :src="miniIcon" class="h-4 w-4 inline-block" alt="Mini">
    </button>
    <span class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-success border border-ui-border" @click="appWindow.minimize()">
      <img :src="minimizeIcon" class="h-6 w-6 inline-block" alt="Minimize">
    </span>
    <span class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-warning border border-ui-border" @click="appWindow.toggleMaximize()">
      <img :src="maximizeIcon" class="h-6 w-6 inline-block" alt="Maximize">
    </span>
    <span class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-error border border-ui-border" @click="appWindow.close()">
      <img :src="closeIcon" class="h-6 w-6 inline-block" alt="Close">
    </span>
  </div>
</template>