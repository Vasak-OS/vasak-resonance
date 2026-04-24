<script lang="ts" setup>
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import AppWindowShell from '@/components/layout/AppWindowShell.vue';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import ResonanceSidebar from '@/components/layout/ResonanceSidebar.vue';
import NowPlayingTopBar from '@/components/player/NowPlayingTopBar.vue';
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
	await playerStore.initProgressListener();
	await playerStore.initMprisNextListener();
	await playerStore.initMprisPreviousListener();
	await playerStore.initMprisStopListener();
});

onUnmounted(() => {
	playerStore.disposeProgressListener();
	playerStore.disposeMprisNextListener();
	playerStore.disposeMprisPreviousListener();
	playerStore.disposeMprisStopListener();
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

			<Transition
				enter-active-class="transition-all duration-200 ease-out"
				enter-from-class="opacity-0 -translate-y-2"
				leave-active-class="transition-all duration-150 ease-in"
				leave-to-class="opacity-0 -translate-y-2"
			>
				<div
					v-if="playerStore.globalBadgeMessage"
					class="pointer-events-none absolute left-1/2 top-2 z-30 -translate-x-1/2 rounded-corner border border-primary/35 bg-primary/12 px-3 py-2 text-xs font-medium text-primary backdrop-blur-sm"
				>
					{{ playerStore.globalBadgeMessage }}
				</div>
			</Transition>

			<div class="relative flex h-full w-full flex-col gap-3 md:flex-row">
				<ResonanceSidebar v-model="selectedSection" />

				<div class="min-h-0 min-w-0 flex-1 flex h-full flex-col gap-2 overflow-hidden">

						<div class="min-h-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80">
							<PlayerControls v-if="selectedSection === 'home'" class="h-full w-full" />
							<AlbumsView v-else-if="selectedSection === 'albums'" />
							<FavoritesView v-else-if="selectedSection === 'favorites'" />
							<PlaylistsView v-else-if="selectedSection === 'playlists'" />
							<PlayerControls v-else class="h-full w-full" />
						</div>
						<NowPlayingTopBar class="shrink-0" />
				</div>
			</div>
		</main>
	</AppWindowShell>
</template>
