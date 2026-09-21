<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SideButton } from '@vasakgroup/vue-libvasak';
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MainTransportControls from '@/components/player/transport/MainTransportControls.vue';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { fetchAlbumCover } from '@/services/album-cover.service';
import { usePlayerStore } from '@/stores/player';
import { siguienteRepeticion } from '@/stores/playerQueue';
import { useSettingsStore } from '@/stores/settings';

const { t } = useI18n();
const playerStore = usePlayerStore();
const settingsStore = useSettingsStore();
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
			// La portada trae su color: lo calculó Rust con los bytes que ya
			// tenía, en lugar de que esta ventana decodifique la imagen otra vez.
			const portada = await fetchAlbumCover(track.artist, track.album);
			if (playerStore.currentTrack?.path !== newPath) {
				return;
			}

			if (playerStore.currentTrack?.cover_data_url) {
				fetchedCoverUrl.value = '';
				return;
			}

			fetchedCoverUrl.value = portada.cover_data_url;

			if (portada.cover_data_url && !track.cover_data_url) {
				playerStore.setCurrentTrackVisuals(portada.cover_data_url, portada.dominant_color || null);
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
			<SideButton
				v-for="section in sections"
				:key="section.id"
				:label="section.label"
				:icon="section.icon"
				:active="selectedSection === section.id"
				@click="onSelectSection(section.id)"
			/>
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
				:repeticion="settingsStore.repeticion"
				:aleatorio="settingsStore.aleatorio"
				@prev="playerStore.playPreviousTrack"
				@toggle="playerStore.togglePlayPause"
				@next="playerStore.advanceQueue"
				@repeat="settingsStore.setRepeticion(siguienteRepeticion(settingsStore.repeticion))"
				@shuffle="settingsStore.setAleatorio(!settingsStore.aleatorio)"
			/>
		</section>
	</aside>
</template>