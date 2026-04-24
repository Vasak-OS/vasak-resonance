<script lang="ts" setup>
import { computed } from 'vue';
import AppWindowShell from '@/components/layout/AppWindowShell.vue';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import ResonanceSidebar from '@/components/layout/ResonanceSidebar.vue';
import PlayerControls from '@/components/player/PlayerControls.vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
import { audioDropDirective } from '@/directives/audioDrop';
import { usePlayerStore } from '@/stores/player';

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
			class="relative flex-1 overflow-hidden p-1 pt-0"
		>
			<AudioDropOverlay :is-active="playerStore.isDragOver" />

			<div class="relative flex h-full w-full flex-col gap-3 md:flex-row">
				<ResonanceSidebar />

				<div class="min-h-0 min-w-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/70">
					<PlayerControls class="h-full w-full" />
				</div>
			</div>
		</main>
	</AppWindowShell>
</template>
