<script lang="ts" setup>
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import AppWindowShell from '@/components/layout/AppWindowShell.vue';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import ResonanceSidebar from '@/components/layout/ResonanceSidebar.vue';
import PlayerControls from '@/components/player/PlayerControls.vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
import AlbumsView from '@/components/views/AlbumsView.vue';
import FavoritesView from '@/components/views/FavoritesView.vue';
import PlaylistsView from '@/components/views/PlaylistsView.vue';
import { audioDropDirective } from '@/directives/audioDrop';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
const selectedSection = ref('home');
const appIcon = ref('');

const dropBinding = computed(() => ({
	onFilesDropped: (paths: string[]) => playerStore.handleDroppedPaths(paths),
	onDragStateChange: (dragging: boolean) => playerStore.setDragOver(dragging),
}));

const vAudioDrop = audioDropDirective;

onMounted(async () => {
	appIcon.value = await getIconSource('applications-multimedia');
});
</script>
<template>
	<AppWindowShell>
		<TopBarComponent>
			<div><img :src="appIcon" class="w-8 h-8" alt="Icono de la aplicación"></div>
			<div class="text-lg font-semibold">Resonance</div>
			<div></div>
		</TopBarComponent>

		<main
			v-audio-drop="dropBinding"
			class="relative flex-1 overflow-hidden p-1 pt-0"
		>
			<AudioDropOverlay :is-active="playerStore.isDragOver" />

			<div class="relative flex h-full w-full flex-col gap-3 md:flex-row">
				<ResonanceSidebar v-model="selectedSection" />

				<div class="min-h-0 min-w-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/70">
					<PlayerControls v-if="selectedSection === 'home'" class="h-full w-full" />
					<AlbumsView v-else-if="selectedSection === 'albums'" />
					<FavoritesView v-else-if="selectedSection === 'favorites'" />
					<PlaylistsView v-else-if="selectedSection === 'playlists'" />
					<PlayerControls v-else class="h-full w-full" />
				</div>
			</div>
		</main>
	</AppWindowShell>
</template>
