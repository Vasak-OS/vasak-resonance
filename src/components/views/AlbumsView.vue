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
	SelectField,
	type SelectOption,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import { DynamicScroller, DynamicScrollerItem } from 'vue-virtual-scroller';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { useVisibleColumns } from '@/composables/useVisibleColumns';
import { fetchAlbumCover } from '@/services/album-cover.service';
import { usePlayerStore } from '@/stores/player';
import { type Album, groupIntoAlbums } from '@/tools/albums';
import { inRows } from '@/tools/lists';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();
const searchQuery = ref('');
const artistFilter = ref('all');
const sortBy = ref('album-asc');

// Cache for fetched cover URLs per artist-album key
const coverUrlCache: Ref<Record<string, string>> = ref({});

const normalize = (value: string) => value.trim().toLowerCase();

const groupedAlbums = computed(() =>
	groupIntoAlbums(playerStore.trackCacheList, t('common.unknownTrack'))
);

// Fetch covers for all albums on mount
const fetchAllCovers = async () => {
	for (const album of groupedAlbums.value) {
		const cacheKey = `${album.artist}|${album.album}`;
		if (!coverUrlCache.value[cacheKey]) {
			try {
				const cover = await fetchAlbumCover(album.artist, album.album);
				if (cover.cover_data_url) {
					coverUrlCache.value[cacheKey] = cover.cover_data_url;
				}
			} catch {
				console.debug(`Failed to fetch cover for ${album.artist} - ${album.album}`);
			}
		}
	}
};

// Getter for cover URL with fallback
const getCoverUrl = (album: Album): string => {
	const cacheKey = `${album.artist}|${album.album}`;
	return coverUrlCache.value[cacheKey] || album.cover || '';
};

const albumArtistOptions = computed(() => {
	const values = new Set(groupedAlbums.value.map((album) => album.artist));
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

const filteredAlbums = computed(() => {
	const query = normalize(searchQuery.value);
	const base = groupedAlbums.value.filter((album) => {
		if (artistFilter.value !== 'all' && album.artist !== artistFilter.value) {
			return false;
		}

		if (!query) {
			return true;
		}

		const matchAlbum = normalize(album.album).includes(query);
		const matchArtist = normalize(album.artist).includes(query);
		const matchTrack = album.tracks.some(
			(track) => normalize(track.title).includes(query) || normalize(track.artist).includes(query)
		);

		return matchAlbum || matchArtist || matchTrack;
	});

	return [...base].sort((left, right) => {
		switch (sortBy.value) {
			case 'album-desc':
				return right.album.localeCompare(left.album);
			case 'tracks-desc':
				return right.tracks.length - left.tracks.length;
			case 'tracks-asc':
				return left.tracks.length - right.tracks.length;
			default:
				return left.album.localeCompare(right.album);
		}
	});
});

/**
 * Las columnas que dibujaba `grid sm:grid-cols-2 xl:grid-cols-3`, ahora dichas
 * a mano porque el scroller coloca las filas él y no puede leerlas del CSS.
 */
const columns = useVisibleColumns([
	{ from: 640, columns: 2 },
	{ from: 1280, columns: 3 },
]);

const albumRows = computed(() => inRows(filteredAlbums.value, columns.value, (album) => album.key));

/**
 * Lo que el scroller supone que mide una fila hasta medirla de verdad.
 *
 * Es una estimación y no un compromiso: `DynamicScroller` mide cada fila al
 * dibujarla y corrige. Sale de medir la tarjeta en WebKitGTK, que va de 532 px
 * a 606 según el ancho; el mínimo es el de las tarjetas más anchas.
 */
const MIN_CARD_HEIGHT = 532;

const artistSelectOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('common.all'), value: 'all' },
	...albumArtistOptions.value.map((artist) => ({ label: artistLabel(artist), value: artist })),
]);

const sortOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('sort.albumAsc'), value: 'album-asc' },
	{ label: t('sort.albumDesc'), value: 'album-desc' },
	{ label: t('sort.mostTracks'), value: 'tracks-desc' },
	{ label: t('sort.fewestTracks'), value: 'tracks-asc' },
]);

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

onMounted(async () => {
	await playerStore.ensureMetadataForFavorites();

	// Fetch album covers from cache/APIs
	await fetchAllCovers();
});

const onPlayTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

const onQueueAlbum = (paths: string[]) => {
	const [firstPath] = paths;
	if (firstPath) {
		const metadata = playerStore.getTrackMetadata(firstPath);
		playerStore.showGlobalBadge(
			t('albums.queuedAlbum').replace('{0}', () => albumLabel(metadata?.album))
		);
	}

	playerStore.enqueuePaths(paths);
};

const onPlayAlbum = async (paths: string[]) => {
	const [firstPath] = paths;
	if (firstPath) {
		const metadata = playerStore.getTrackMetadata(firstPath);
		playerStore.showGlobalBadge(
			t('albums.playingAlbum').replace('{0}', () => albumLabel(metadata?.album))
		);
	}

	await playerStore.playAlbum(paths);
};
</script>

<template>
	<!-- `@container`: los filtros se ponen en fila cuando la vista tiene lugar
	     (antes `lg:`, 1024 de ventana; 44 rem de vista es el mismo corte con la
	     barra al costado). Las columnas de la cuadrícula las cuenta
	     `useVisibleColumns`, con un `ResizeObserver`. -->
	<section class="@container flex h-full flex-col gap-4 overflow-hidden p-4">
		<PageHeader class="mb-4" :eyebrow="t('albums.eyebrow')" :title="t('albums.title')" as="h2" />

		<div class="mb-4 grid gap-3 @min-[44rem]:grid-cols-[1.4fr_0.8fr_0.8fr]">
			<FormGroup :label="t('common.search')" variant="eyebrow">
				<SearchField v-model="searchQuery" :label="t('common.search')" :placeholder="t('albums.searchPlaceholder')" />
			</FormGroup>

			<FormGroup v-slot="{ id }" :label="t('common.artist')" variant="eyebrow">
				<SelectField v-bind="{ id }" v-model="artistFilter" :options="artistSelectOptions" />
			</FormGroup>

			<FormGroup v-slot="{ id }" :label="t('common.sortBy')" variant="eyebrow">
				<SelectField v-bind="{ id }" v-model="sortBy" :options="sortOptions" />
			</FormGroup>
		</div>

		<EmptyState v-if="filteredAlbums.length === 0" :title="t('albums.empty')" bordered />

		<!-- Un solo menú para toda la cuadrícula; cada canción de la vista previa
		     dice cuál es la suya con `data-track-path`. -->
		<div v-else class="min-h-0 flex-1 overflow-hidden" @contextmenu="onTrackContextMenu">
			<!-- Se virtualiza **por filas**: una fila de la cuadrícula es un
			     elemento del scroller y adentro va el `grid` de siempre.
			     `DynamicScroller` y no `RecycleScroller` porque la tarjeta no
			     tiene un alto fijo: medida en WebKitGTK, la misma tarjeta va de
			     606 px a 280 de ancho a 532 px a 720. Un alto declarado a mano
			     recortaría canciones de la vista previa en la mitad de los
			     anchos, y nada avisaría.

			     La tarjeta mide lo mismo que antes de pasar a los componentes de
			     la librería (518 px a 1200 de ventana, 534 a 1400, medidos en el
			     banco): los botones con texto van en filas de 34 px y las
			     canciones de la vista previa en 38, que es lo que medían. -->
			<DynamicScroller
				:items="albumRows"
				:min-item-size="MIN_CARD_HEIGHT"
				key-field="key"
				class="h-full overflow-y-auto"
				v-slot="{ item: row, index, active }"
			>
				<DynamicScrollerItem
					:item="row"
					:active="active"
					:index="index"
					:size-dependencies="[columns, row.items.length]"
				>
					<div
						class="mb-4 grid gap-4"
						:style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }"
					>
						<Panel v-for="album in row.items" :key="album.key" as="article">
							<!-- La tapa, cuadrada y entera, en la misma franja de 11 rem
							     que antes ocupaba recortada a lo ancho. -->
							<div class="mb-3 flex h-44 justify-center">
								<div class="aspect-square h-full max-w-full">
									<CoverArt
										:src="getCoverUrl(album) || null"
										:alt="albumLabel(album.album)"
										:fallback-text="t('common.noCover')"
									/>
								</div>
							</div>
							<p class="truncate text-base font-semibold text-tx-main">{{ albumLabel(album.album) }}</p>
							<p class="truncate text-sm text-tx-muted">{{ artistLabel(album.artist) }}</p>
							<p class="mt-2 text-xs uppercase tracking-[0.12em] text-tx-muted">
								{{ t(album.tracks.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
									.replace('{0}', String(album.tracks.length)) }}
							</p>

							<div class="mt-3 grid auto-rows-[minmax(2.125rem,auto)] grid-cols-2 gap-2">
								<ActionButton
									:label="t('albums.playAlbum')"
									icon="media-playback-start"
									full-width
									@click="onPlayAlbum(album.tracks.map((track) => track.path))"
								/>
								<ActionButton
									:label="t('albums.queueAlbum')"
									icon="media-track-add-amarok"
									variant="secondary"
									full-width
									@click="onQueueAlbum(album.tracks.map((track) => track.path))"
								/>
							</div>

							<ul class="mt-3 grid gap-1.5">
								<li
									v-for="track in album.tracksPreview"
									:key="track.path"
									:data-track-path="track.path"
									class="flex min-h-[2.375rem] min-w-0 items-center gap-2 rounded-corner-m border border-transparent px-2 py-1 transition-colors duration-200 ease-ui hover:bg-ui-hover"
								>
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
				</DynamicScrollerItem>
			</DynamicScroller>
		</div>
	</section>
</template>
