<script lang="ts" setup>
import { computed } from 'vue';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import AppWindowShell from '@/components/layout/AppWindowShell.vue';
import PlayerControls from '@/components/player/PlayerControls.vue';
import { audioDropDirective } from '@/directives/audioDrop';
import { usePlayerStore } from '@/stores/player';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';

const playerStore = usePlayerStore();

const dropBinding = computed(() => ({
	onFilesDropped: (paths: string[]) => playerStore.handleDroppedPaths(paths),
	onDragStateChange: (dragging: boolean) => playerStore.setDragOver(dragging),
}));

const vAudioDrop = audioDropDirective;
</script>
<template>
	<AppWindowShell>
		<TopBarComponent />

		<main
			v-audio-drop="dropBinding"
			class="relative flex-1 overflow-hidden p-4"
		>
			<AudioDropOverlay :is-active="playerStore.isDragOver" />

			<div class="relative flex h-full w-full items-center justify-center">
				<PlayerControls class="w-full" />
			</div>
		</main>
	</AppWindowShell>
</template>
