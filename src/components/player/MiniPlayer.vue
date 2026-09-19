<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { WindowFrame } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import PlayerBackground from '@/components/player/PlayerBackground.vue';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MiniTransportControls from '@/components/player/transport/MiniTransportControls.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { fetchAlbumCover } from '@/services/album-cover.service';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
const fetchedCoverUrl = ref<string>('');

useConfigSync();

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});

const trackSubtitle = useTrackSubtitle({
	currentTrack: () => playerStore.currentTrack,
});

const coverSrc = computed(() => {
	// First try embedded cover
	if (playerStore.currentTrack?.cover_data_url) {
		return playerStore.currentTrack.cover_data_url;
	}
	// Fall back to fetched cover from cache/APIs
	return fetchedCoverUrl.value;
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
			console.debug('Failed to fetch cover for current track in miniplayer');
			fetchedCoverUrl.value = '';
		}
	},
	{ immediate: true }
);

const togglePlayback = async () => {
	await playerStore.togglePlayPause();
};

const openMainWindow = async () => {
	await toggleMainAndMiniPlayer();
};

onMounted(async () => {
	await playerStore.initProgressListener();
});

onUnmounted(() => {
	playerStore.disposeProgressListener();
});
</script>

<template>
	<!-- Sin barra. Esto es un widget, no una ventana: trescientos sesenta
	     píxeles por ciento veinte, siempre encima, sin redimensionar. Una barra
	     con botones se comería un tercio del alto, y los botones no
	     significarían nada: no hay nada que minimizar ni maximizar, y para
	     volver a la ventana grande está el botón del transporte, que es donde
	     la mano ya lo busca.

	     Del marco compartido se queda con lo que sí hace falta: que el borde,
	     la esquina y el fondo sean los mismos que los del resto del escritorio.
	     El fondo es el del marco y no uno propio: dos utilidades de fondo sobre
	     el mismo elemento las desempata el orden del CSS generado, no el del
	     atributo, así que un `bg-ui-bg/90` encima del `/80` del marco gana o
	     pierde según el día. Y una ventana con otro fondo que las demás es
	     justo lo que esto vino a sacar. -->
	<WindowFrame hide-bar>
		<div class="relative min-h-0 min-w-0 flex-1 overflow-hidden p-3">
		<PlayerBackground />

		<div class="relative z-10 flex h-full flex-col">
			<div class="flex min-h-0 flex-1 items-center gap-2">
				<TrackMetaCard
					class="min-w-0 flex-1"
					:title="trackTitle"
					:subtitle="trackSubtitle"
					:cover-src="coverSrc"
					placeholder-text="VR"
					title-class="text-primary"
				/>

				<MiniTransportControls
					:has-track="playerStore.hasTrack"
					:has-next-track="playerStore.hasNextTrack"
					:is-playing="playerStore.isPlaying"
					:is-paused="playerStore.isPaused"
					:busy="playerStore.busy"
					:next-label="t('transport.next')"
					:open-label="t('miniPlayer.backToWindow')"
					@toggle="togglePlayback"
					@next="playerStore.advanceQueue"
					@open="openMainWindow"
				/>
			</div>

			<PlaybackWaves class="mt-2" :steps="72" bar-height="h-3" :floor-paused="2" :floor-playing="4" :amplitude="7" />
		</div>
		</div>
	</WindowFrame>
</template>
