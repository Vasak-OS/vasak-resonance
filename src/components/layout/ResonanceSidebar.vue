<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MainTransportControls from '@/components/player/transport/MainTransportControls.vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { extractDominantColorFromDataUrl, fetchAlbumCover } from '@/services/album-cover.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
const router = useRouter();
const route = useRoute();

// Las etiquetas se recalculan con el idioma, así que la lista es un `computed`
// y la navegación ya no puede llevar `v-once`: con la plantilla congelada, el
// primer pintado —que ocurre antes de que lleguen las traducciones— dejaba los
// nombres de las secciones puestos para siempre.
const sections = computed(() => [
	{ id: 'home', label: t('sidebar.home'), icon: 'go-home-symbolic' },
	{ id: 'albums', label: t('sidebar.albums'), icon: 'folder-music-symbolic' },
	{ id: 'favorites', label: t('sidebar.favorites'), icon: 'starred-symbolic' },
	{ id: 'playlists', label: t('sidebar.playlists'), icon: 'view-list-symbolic' },
	{ id: 'radios', label: t('sidebar.radios'), icon: 'media-playback-start-symbolic' },
	{ id: 'settings', label: t('sidebar.settings'), icon: 'preferences-system-symbolic' },
]);

const homeIcon = useReactiveIcon('go-home-symbolic');
const albumsIcon = useReactiveIcon('folder-music-symbolic');
const favoritesIcon = useReactiveIcon('starred-symbolic');
const playlistsIcon = useReactiveIcon('view-list-symbolic');
const radiosIcon = useReactiveIcon('media-playback-start-symbolic');
const settingsIcon = useReactiveIcon('preferences-system-symbolic');

const iconSources = computed(() => ({
	home: homeIcon.value,
	albums: albumsIcon.value,
	favorites: favoritesIcon.value,
	playlists: playlistsIcon.value,
	radios: radiosIcon.value,
	settings: settingsIcon.value,
}));

const fetchedCoverUrl = ref<string>('');

const selectedSection = computed(() => (typeof route.name === 'string' ? route.name : 'home'));

const coverArt = computed(() => {
	// First try embedded cover
	if (playerStore.currentTrack?.cover_data_url) {
		return playerStore.currentTrack.cover_data_url;
	}
	// Fall back to fetched cover from cache/APIs
	return fetchedCoverUrl.value;
});

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});

const trackSubtitle = useTrackSubtitle({
	currentTrack: () => playerStore.currentTrack,
});

// Watch for track changes and fetch cover if needed
watch(
	() => playerStore.currentTrack?.path,
	async (newPath) => {
		if (!newPath) {
			fetchedCoverUrl.value = '';
			return;
		}

		const track = playerStore.currentTrack;
		if (!track) {
			fetchedCoverUrl.value = '';
			return;
		}

		// If track has embedded cover, don't fetch
		if (track.cover_data_url) {
			fetchedCoverUrl.value = '';
			return;
		}

		// Try to fetch cover from cache/APIs
		try {
			const url = await fetchAlbumCover(track.artist, track.album);
			if (playerStore.currentTrack?.path !== newPath) {
				return;
			}

			if (playerStore.currentTrack?.cover_data_url) {
				fetchedCoverUrl.value = '';
				return;
			}

			fetchedCoverUrl.value = url;

			if (url && !track.cover_data_url) {
				const dominantColor = await extractDominantColorFromDataUrl(url);
				if (playerStore.currentTrack?.path === newPath) {
					playerStore.setCurrentTrackVisuals(url, dominantColor);
				}
			}
		} catch (error) {
			console.debug('Failed to fetch cover for current track');
			fetchedCoverUrl.value = '';
		}
	},
	{ immediate: true }
);

const onSelectSection = async (id: string) => {
	if (selectedSection.value === id) {
		return;
	}

	await router.push({ name: id });
};
</script>

<template>
	<aside class="flex w-full shrink-0 flex-col rounded-corner border border-ui-border bg-ui-bg/80 p-2 md:w-72">
		<header class="border-b border-ui-border px-2 pb-3 pt-1">
			<p class="text-xs uppercase tracking-[0.12em] text-tx-muted">{{ t('sidebar.sectionTitle') }}</p>
			<p class="text-sm font-semibold text-tx-main">{{ t('sidebar.library') }}</p>
		</header>

		<nav class="flex-1 space-y-2 overflow-y-auto px-1 py-3">
			<button
				v-for="section in sections"
				:key="section.id"
				type="button"
				class="flex w-full items-center gap-3 rounded-corner border px-3 py-2 text-left text-sm transition-all duration-200"
				:class="[
					selectedSection === section.id
						? 'border-secondary bg-primary/15 text-tx-main'
						: 'border-transparent bg-ui-bg/30 text-tx-muted hover:border-ui-border hover:bg-ui-surface/70 hover:text-tx-main',
					'cursor-pointer',
				]"
				@click="onSelectSection(section.id)"
			>
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-corner border border-ui-border bg-ui-bg/55">
					<img
						v-if="iconSources[section.id as keyof typeof iconSources]"
						:src="iconSources[section.id as keyof typeof iconSources]"
						:alt="section.label"
						class="h-5 w-5 object-contain"
					/>
					<span v-else class="text-xs font-semibold">{{ section.label.charAt(0) }}</span>
				</span>
				<span class="flex-1">{{ section.label }}</span>
			</button>
		</nav>

		<section class="mt-2 rounded-corner border border-ui-border bg-ui-surface/40 p-3">
			<TrackMetaCard
				:title="trackTitle"
				:subtitle="trackSubtitle"
				:cover-src="coverArt"
				variant="stacked"
				placeholder-text="VR"
			/>

			<MainTransportControls
				:has-track="playerStore.hasTrack"
				:has-next-track="playerStore.hasNextTrack"
				:busy="playerStore.busy"
				:is-paused="playerStore.isPaused"
				:next-action-label="playerStore.nextActionLabel"
				@prev="playerStore.playPreviousTrack"
				@toggle="playerStore.togglePlayPause"
				@next="playerStore.advanceQueue"
			/>
		</section>
	</aside>
</template>