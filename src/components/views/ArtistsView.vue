<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	CoverArt,
	EmptyState,
	FormGroup,
	PageHeader,
	Panel,
	SearchField,
} from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { usePlayerStore } from '@/stores/player';
import { groupByArtist } from '@/tools/artists';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();
const searchQuery = ref('');
/** El artista que se está mirando, o `null` para la lista. */
const openArtistKey = ref<string | null>(null);

const normalize = (value: string) => value.trim().toLowerCase();

const artists = computed(() => groupByArtist(playerStore.trackCacheList, t('common.unknownTrack')));

const filteredArtists = computed(() => {
	const query = normalize(searchQuery.value);
	if (!query) {
		return artists.value;
	}
	// Se busca por el nombre traducido además del crudo: quien ve «Artista
	// desconocido» en pantalla espera encontrarlo escribiendo eso.
	return artists.value.filter(
		(artist) =>
			normalize(artist.name).includes(query) || normalize(artistLabel(artist.name)).includes(query)
	);
});

const openArtist = computed(
	() => artists.value.find((candidate) => candidate.key === openArtistKey.value) ?? null
);

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const open = (key: string) => {
	openArtistKey.value = key;
};

const back = () => {
	openArtistKey.value = null;
};

const onPlayTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

const onQueue = (paths: string[], name: string) => {
	playerStore.showGlobalBadge(t('artists.queued').replace('{0}', () => name));
	playerStore.enqueuePaths(paths);
};

const onPlay = async (paths: string[], name: string) => {
	playerStore.showGlobalBadge(t('artists.playing').replace('{0}', () => name));
	await playerStore.playAlbum(paths);
};

/** Todo lo del artista, en el orden en que se muestra: disco por disco. */
const artistTracks = (key: string): string[] => {
	const found = artists.value.find((candidate) => candidate.key === key);
	if (!found) {
		return [];
	}
	return found.albums.flatMap((album) => album.tracks.map((track) => track.path));
};
</script>

<template>
	<!-- `@container`: las columnas de la cuadrícula siguen al ancho de la vista
	     y no al de la pantalla. Los cortes son los de `sm:` (640) y `xl:`
	     (1280) de ventana, medidos en la vista con la barra al costado:
	     28 rem y 60 rem. -->
	<section class="@container h-full overflow-y-auto p-4">
		<PageHeader class="mb-4" :eyebrow="t('artists.eyebrow')" :title="t('artists.title')" as="h2" />

		<!-- La lista de artistas -->
		<template v-if="!openArtist">
			<FormGroup class="mb-4" :label="t('common.search')" variant="eyebrow">
				<SearchField v-model="searchQuery" :label="t('common.search')" :placeholder="t('artists.searchPlaceholder')" />
			</FormGroup>

			<EmptyState v-if="filteredArtists.length === 0" :title="t('artists.empty')" bordered />

			<div v-else class="grid gap-4 @min-[28rem]:grid-cols-2 @min-[60rem]:grid-cols-3">
				<Panel v-for="candidate in filteredArtists" :key="candidate.key" as="article">
					<button
						type="button"
						class="block w-full rounded-corner-m text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ui-focus"
						:aria-label="t('artists.open').replace('{0}', () => artistLabel(candidate.name))"
						@click="open(candidate.key)"
					>
						<!-- La tapa, cuadrada y entera, en la misma franja de 11 rem. -->
						<span class="mb-3 flex h-44 justify-center">
							<span class="aspect-square h-full max-w-full">
								<CoverArt :src="candidate.cover" :alt="artistLabel(candidate.name)" :fallback-text="t('common.noCover')" />
							</span>
						</span>
						<span class="block truncate text-base font-semibold text-tx-main">{{ artistLabel(candidate.name) }}</span>
						<span class="mt-2 block text-xs uppercase tracking-[0.12em] text-tx-muted">
							{{ t(candidate.albums.length === 1 ? 'artists.albumCountOne' : 'artists.albumCountOther')
								.replace('{0}', String(candidate.albums.length)) }}
							·
							{{ t(candidate.tracks.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
								.replace('{0}', String(candidate.tracks.length)) }}
						</span>
					</button>

					<div class="mt-3 grid grid-cols-2 gap-2">
						<ActionButton
							:label="t('artists.playAll')"
							icon="media-playback-start"
							full-width
							@click="onPlay(artistTracks(candidate.key), artistLabel(candidate.name))"
						/>
						<ActionButton
							:label="t('artists.queueAll')"
							icon="media-track-add-amarok"
							variant="secondary"
							full-width
							@click="onQueue(artistTracks(candidate.key), artistLabel(candidate.name))"
						/>
					</div>
				</Panel>
			</div>
		</template>

		<!-- Un artista, con sus discos -->
		<template v-else>
			<div class="mb-4 flex min-w-0 flex-wrap items-center gap-3">
				<ActionButton :label="t('artists.back')" icon="go-previous-symbolic" variant="secondary" @click="back" />
				<h3 class="min-w-0 flex-1 truncate text-lg font-semibold text-tx-main">
					{{ artistLabel(openArtist.name) }}
				</h3>
				<ActionButton
					:label="t('artists.playAll')"
					icon="media-playback-start"
					@click="onPlay(artistTracks(openArtist.key), artistLabel(openArtist.name))"
				/>
			</div>

			<!-- Un solo menú para todo; cada pista dice cuál es la suya. -->
			<div class="grid gap-4" @contextmenu="onTrackContextMenu">
				<Panel v-for="album in openArtist.albums" :key="album.key" as="article">
					<div class="mb-3 flex min-w-0 flex-wrap items-center gap-3">
						<CoverArt :src="album.cover || null" :alt="albumLabel(album.album)" size="md" />
						<div class="min-w-0 flex-1">
							<p class="truncate text-base font-semibold text-tx-main">{{ albumLabel(album.album) }}</p>
							<p class="text-xs uppercase tracking-[0.12em] text-tx-muted">
								{{ t(album.tracks.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
									.replace('{0}', String(album.tracks.length)) }}
							</p>
						</div>
						<ActionButton
							:label="t('albums.queueAlbum')"
							icon="media-track-add-amarok"
							variant="secondary"
							@click="onQueue(album.tracks.map((track) => track.path), albumLabel(album.album))"
						/>
					</div>

					<ul class="grid gap-1.5">
						<li
							v-for="track in album.tracks"
							:key="track.path"
							:data-track-path="track.path"
							class="flex min-w-0 items-center gap-2 rounded-corner-m border border-transparent px-2 py-1 transition-colors duration-200 ease-ui hover:bg-ui-hover"
						>
							<span v-if="track.track_no > 0" class="w-6 shrink-0 text-right text-xs tabular-nums text-tx-muted">
								{{ track.track_no }}
							</span>
							<p class="min-w-0 flex-1 truncate text-xs text-tx-muted">
								{{ track.title || extractTrackName(track.path) }}
							</p>
							<ActionButton
								label=""
								:icon-alt="t('common.play')"
								:title="t('common.play')"
								icon="media-playback-start"
								variant="ghost"
								size="sm"
								@click="onPlayTrack(track.path)"
							/>
						</li>
					</ul>
				</Panel>
			</div>
		</template>
	</section>
</template>
