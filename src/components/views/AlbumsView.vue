<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import { DynamicScroller, DynamicScrollerItem } from 'vue-virtual-scroller';
import LabeledField from '@/components/layout/LabeledField.vue';
import { useColumnasVisibles } from '@/composables/useColumnasVisibles';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { fetchAlbumCover } from '@/services/album-cover.service';
import { usePlayerStore } from '@/stores/player';
import { agruparEnDiscos } from '@/tools/albumes';
import { enFilas } from '@/tools/listas';

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
	agruparEnDiscos(playerStore.trackCacheList, t('common.unknownTrack'))
);

// Fetch covers for all albums on mount
const fetchAllCovers = async () => {
	for (const album of groupedAlbums.value) {
		const cacheKey = `${album.artist}|${album.album}`;
		if (!coverUrlCache.value[cacheKey]) {
			try {
				const portada = await fetchAlbumCover(album.artist, album.album);
				if (portada.cover_data_url) {
					coverUrlCache.value[cacheKey] = portada.cover_data_url;
				}
			} catch (error) {
				console.debug(`Failed to fetch cover for ${album.artist} - ${album.album}`);
			}
		}
	}
};

// Getter for cover URL with fallback
const getCoverUrl = (album: any): string => {
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
const columnas = useColumnasVisibles([
	{ desde: 640, columnas: 2 },
	{ desde: 1280, columnas: 3 },
]);

const filasDeDiscos = computed(() =>
	enFilas(filteredAlbums.value, columnas.value, (album) => album.key)
);

/**
 * Lo que el scroller supone que mide una fila hasta medirla de verdad.
 *
 * Es una estimación y no un compromiso: `DynamicScroller` mide cada fila al
 * dibujarla y corrige. Sale de medir la tarjeta en WebKitGTK, que va de 532 px
 * a 606 según el ancho; el mínimo es el de las tarjetas más anchas.
 */
const ALTO_MINIMO_DE_TARJETA = 532;

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
	<section class="flex h-full flex-col gap-4 overflow-hidden p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">{{ t('albums.eyebrow') }}</p>
			<h2 class="text-lg font-semibold text-tx-main">{{ t('albums.title') }}</h2>
		</div>

		<div class="mb-4 grid gap-3 lg:grid-cols-[1.4fr_0.8fr_0.8fr]">
			<LabeledField :label="t('common.search')">
				<input
					v-model="searchQuery"
					type="search"
					:placeholder="t('albums.searchPlaceholder')"
					class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 placeholder:text-tx-muted/70 focus:border-primary/50"
				/>
			</LabeledField>

			<LabeledField :label="t('common.artist')">
				<select v-model="artistFilter" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 focus:border-primary/50">
					<option value="all">{{ t('common.all') }}</option>
					<option v-for="artist in albumArtistOptions" :key="artist" :value="artist">{{ artistLabel(artist) }}</option>
				</select>
			</LabeledField>

			<LabeledField :label="t('common.sortBy')">
				<select v-model="sortBy" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 focus:border-primary/50">
					<option value="album-asc">{{ t('sort.albumAsc') }}</option>
					<option value="album-desc">{{ t('sort.albumDesc') }}</option>
					<option value="tracks-desc">{{ t('sort.mostTracks') }}</option>
					<option value="tracks-asc">{{ t('sort.fewestTracks') }}</option>
				</select>
			</LabeledField>
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
			     anchos, y nada avisaría. -->
			<DynamicScroller
				:items="filasDeDiscos"
				:min-item-size="ALTO_MINIMO_DE_TARJETA"
				key-field="clave"
				class="h-full overflow-y-auto"
				v-slot="{ item: fila, index, active }"
			>
				<DynamicScrollerItem
					:item="fila"
					:active="active"
					:index="index"
					:size-dependencies="[columnas, fila.elementos.length]"
				>
					<div
						class="mb-4 grid gap-4"
						:style="{ gridTemplateColumns: `repeat(${columnas}, minmax(0, 1fr))` }"
					>
			<article
				v-for="album in fila.elementos"
				:key="album.key"
				class="rounded-corner border border-ui-border bg-ui-bg/80 p-4"
			>
				<div class="mb-3 flex h-44 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
					<img v-if="getCoverUrl(album)" :src="getCoverUrl(album)" :alt="albumLabel(album.album)" class="h-full w-full object-cover" />
					<div v-else class="text-sm font-semibold uppercase tracking-[0.16em] text-tx-muted">{{ t('common.noCover') }}</div>
				</div>
				<p class="truncate text-base font-semibold text-tx-main">{{ albumLabel(album.album) }}</p>
				<p class="truncate text-sm text-tx-muted">{{ artistLabel(album.artist) }}</p>
				<p class="mt-2 text-xs uppercase tracking-[0.12em] text-primary">
					{{ t(album.tracks.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
						.replace('{0}', String(album.tracks.length)) }}
				</p>

				<div class="mt-3 grid grid-cols-2 gap-2">
					<button
						type="button"
						class="inline-flex w-full items-center justify-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90"
						:title="t('albums.playAlbum')"
						:aria-label="t('albums.playAlbum')"
						@click="onPlayAlbum(album.tracks.map((track) => track.path))"
					>
						<ThemeIcon name="media-playback-start" type="symbol" :size="16" />
						{{ t('albums.playAlbum') }}
					</button>
					<button
						type="button"
						class="inline-flex w-full items-center justify-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
						:title="t('albums.queueAlbum')"
						:aria-label="t('albums.queueAlbum')"
						@click="onQueueAlbum(album.tracks.map((track) => track.path))"
					>
						<ThemeIcon name="media-track-add-amarok" type="symbol" :size="16" />
						{{ t('albums.queueAlbum') }}
					</button>
				</div>

				<ul class="mt-3 grid gap-1.5">
					<li
						v-for="track in album.tracksPreview"
						:key="track.path"
						:data-track-path="track.path"
						class="flex min-w-0 items-center gap-2 rounded-corner border border-transparent px-2 py-1 hover:border-ui-border hover:bg-ui-surface/45"
					>
						<p class="min-w-0 flex-1 truncate text-xs text-tx-muted">
							{{ track.title || extractTrackName(track.path) }}
						</p>
						<button
							type="button"
							class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-corner border border-primary/45 bg-primary/10 text-[11px] font-medium text-primary transition-colors duration-200 hover:bg-primary/20"
							:title="t('common.play')"
							:aria-label="t('common.play')"
							@click="onPlayTrack(track.path)"
						>
							<ThemeIcon name="media-playback-start" type="symbol" :size="14" />
							<span class="sr-only">{{ t('common.play') }}</span>
						</button>
					</li>
				</ul>
			</article>
					</div>
				</DynamicScrollerItem>
			</DynamicScroller>
		</div>
	</section>
</template>
