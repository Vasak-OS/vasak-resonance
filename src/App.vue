<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import MiniPlayer from '@/components/player/MiniPlayer.vue';
import { useConfigSync } from '@/composables/useConfigSync';

const isMiniPlayerWindow = getCurrentWindow().label === 'mini-player';

useConfigSync({ useViewTransition: true });
</script>

<template>
	<MiniPlayer v-if="isMiniPlayerWindow" />
	<RouterView v-else />
</template>
