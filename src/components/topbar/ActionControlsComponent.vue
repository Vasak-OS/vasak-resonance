<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import { toggleMainAndMiniPlayer } from '@/services/window.service';

const appWindow = getCurrentWindow();
const miniIcon = useReactiveIcon('screenshot-ui-window');
const closeIcon = useReactiveIcon('window-close');
const minimizeIcon = useReactiveIcon('window-minimize');
const maximizeIcon = useReactiveIcon('window-maximize');

const toggleMiniPlayer = async () => {
	await toggleMainAndMiniPlayer();
};

const closeApp = async () => {
	try {
		// Call backend close command which will shutdown audio first
		await invoke('close_app');
	} catch (error) {
		console.error('Failed to close app:', error);
		// Fallback to direct close if command fails
		await appWindow.close();
	}
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
    <span class="p-1 bg-ui-bg/80 rounded-corner hover:bg-status-error border border-ui-border" @click="closeApp">
      <img :src="closeIcon" class="h-6 w-6 inline-block" alt="Close">
    </span>
  </div>
</template>