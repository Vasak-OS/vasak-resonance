<script setup lang="ts">
import { RecycleScroller } from 'vue-virtual-scroller';
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Badge,
	EmptyState,
	FormGroup,
	ListRow,
	LoadingState,
	PageHeader,
	Panel,
	SearchField,
	SelectField,
	type SelectOption,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useElementWidth } from '@/composables/useElementWidth';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import {
	type DroppedPlaybackTrack,
	type LibraryTrack,
	listLibraryTracks,
	saveLibraryTrack,
	searchLibraryTracks,
} from '@/services/player.service';
import { usePlayerStore } from '@/stores/player';
import { createQueueEntries, elegirSiguiente } from '@/stores/playerQueue';
import { useSettingsStore } from '@/stores/settings';
import { rowActionsWithLabels } from '@/tools/window-layout';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const settingsStore = useSettingsStore();
const { onTrackContextMenu } = useTrackContextMenu();
const libraryTracks = ref<LibraryTrack[]>([]);
const isLoading = ref(false);
const errorMessage = ref('');
const searchQuery = ref('');
const artistFilter = ref('all');
const albumFilter = ref('all');
const sortBy = ref('recent-desc');
const ftsSearchResults = ref<LibraryTrack[] | null>(null);

/**
 * Lo que mide una fila de la lista, contando el hueco de abajo: 84 de alto más
 * los 8 del `mb-2`. El `RecycleScroller` coloca las filas a partir de este
 * número y no las mide, así que si se separa de lo que dibuja el CSS las filas
 * se pisan o dejan huecos, sin que nada falle. Hay una prueba que compara los
 * dos, y el banco lo midió igual antes y después de pasar a `ListRow`.
 */
const ROW_HEIGHT = 92;

/** La lista, para saber si los botones de cada fila entran con su texto. */
const list = ref<HTMLElement | null>(null);
const listWidth = useElementWidth(list);
const labeledActions = computed(() => rowActionsWithLabels(listWidth.value));
let searchDebounceTimer: number | null = null;

const normalize = (value: string) => value.trim().toLowerCase();

const formatDuration = (seconds: number) => {
	const totalSeconds = Math.max(0, Math.floor(seconds || 0));
	const minutes = Math.floor(totalSeconds / 60)
		.toString()
		.padStart(2, '0');
	const remaining = Math.floor(totalSeconds % 60)
		.toString()
		.padStart(2, '0');

	return `${minutes}:${remaining}`;
};

const extractName = (path: string) => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const toLibraryTrack = (
	track: DroppedPlaybackTrack,
	createdAt = new Date().toISOString()
): LibraryTrack => ({
	id: -1,
	path: track.path,
	title: track.title,
	artist: track.artist,
	album: track.album,
	album_artist: track.album_artist,
	track_no: track.track_no,
	duration_seconds: track.duration_seconds,
	created_at: createdAt,
});

// Los centinelas «Unknown Artist» y «Unknown Album» son los que escribe el
// backend y con los que se agrupa y se filtra: quedan como están y se traducen
// al mostrarlos, con `artistLabel` y `albumLabel`.
const sanitizeTrack = (track: LibraryTrack): LibraryTrack => ({
	...track,
	title: track.title?.trim() || extractName(track.path),
	artist: track.artist?.trim() || 'Unknown Artist',
	album: track.album?.trim() || 'Unknown Album',
});

const loadLibrary = async () => {
	isLoading.value = true;
	errorMessage.value = '';
	try {
		libraryTracks.value = await listLibraryTracks();
	} catch (error) {
		errorMessage.value = t('home.libraryLoadError').replace('{0}', () => String(error));
	} finally {
		isLoading.value = false;
	}
};

const syncCachedTracksToDatabase = async () => {
	await Promise.allSettled(playerStore.trackCacheList.map((track) => saveLibraryTrack(track)));
};

const runFtsSearch = async () => {
	const query = normalize(searchQuery.value);
	if (!query) {
		ftsSearchResults.value = null;
		return;
	}

	try {
		const results = await searchLibraryTracks(query, 5000);
		ftsSearchResults.value = results.map(sanitizeTrack);
	} catch (error) {
		console.error('[HomeView] FTS search error:', error);
		ftsSearchResults.value = [];
	}
};

const librarySourceTracks = computed(() => {
	const merged = new Map<string, LibraryTrack>();

	for (const track of libraryTracks.value) {
		merged.set(track.path, sanitizeTrack(track));
	}

	for (const track of playerStore.trackCacheList) {
		if (!merged.has(track.path)) {
			merged.set(track.path, sanitizeTrack(toLibraryTrack(track)));
		}
	}

	return Array.from(merged.values());
});

const artistOptions = computed(() => {
	const values = new Set(
		librarySourceTracks.value
			.map((track) => track.artist?.trim())
			.filter((value): value is string => Boolean(value))
	);
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

const albumOptions = computed(() => {
	const values = new Set(
		librarySourceTracks.value
			.map((track) => track.album?.trim())
			.filter((value): value is string => Boolean(value))
	);
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

const sortedTracks = computed(() => {
	const sourceTracks = ftsSearchResults.value ?? librarySourceTracks.value;
	const filtered = sourceTracks.filter((track) => {
		if (artistFilter.value !== 'all' && track.artist !== artistFilter.value) {
			return false;
		}

		if (albumFilter.value !== 'all' && track.album !== albumFilter.value) {
			return false;
		}

		return true;
	});

	return [...filtered].sort((left, right) => {
		switch (sortBy.value) {
			case 'title-asc':
				return left.title.localeCompare(right.title);
			case 'title-desc':
				return right.title.localeCompare(left.title);
			case 'artist-asc':
				return left.artist.localeCompare(right.artist) || left.title.localeCompare(right.title);
			case 'artist-desc':
				return right.artist.localeCompare(left.artist) || left.title.localeCompare(right.title);
			case 'album-asc':
				return left.album.localeCompare(right.album) || left.title.localeCompare(right.title);
			case 'duration-asc':
				return left.duration_seconds - right.duration_seconds;
			case 'duration-desc':
				return right.duration_seconds - left.duration_seconds;
			default:
				return (
					right.created_at.localeCompare(left.created_at) || left.title.localeCompare(right.title)
				);
		}
	});
});

const playTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

/**
 * Poner todo lo que se está viendo, salteado.
 *
 * Antes esto mezclaba la lista con `sort(() => Math.random() - 0.5)` y la
 * encolaba ya revuelta. Eso tenía dos problemas: un comparador que contesta al
 * azar **no baraja parejo** —el resultado depende de qué algoritmo de
 * ordenamiento use el motor, y las posiciones quedan sesgadas—, y lo que se
 * agregara después entraba en orden igual, porque el aleatorio no era un modo
 * sino una mezcla de una vez.
 *
 * Ahora se prende el modo y la cola se encola en orden: de dónde sale cada
 * canción lo decide `elegirSiguiente`, canción por canción.
 */
const playRandomFiltered = async () => {
	const list = sortedTracks.value ?? [];
	if (list.length === 0) {
		playerStore.globalBadgeMessage = t('home.nothingToPlay');
		return;
	}

	await settingsStore.setAleatorio(true);

	const paths = list.map((track) => track.path);
	// Con qué canción arranca lo decide la misma regla que elige cada una de
	// las siguientes con el aleatorio puesto: una sola forma de sortear.
	const { siguiente: first } = elegirSiguiente(createQueueEntries(paths), null, 'ninguna', true);
	if (!first) {
		return;
	}

	await playerStore.playDropped(first);
	playerStore.enqueuePaths(paths.filter((path) => path !== first));
};

const artistSelectOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('common.all'), value: 'all' },
	...artistOptions.value.map((artist) => ({ label: artistLabel(artist), value: artist })),
]);

const albumSelectOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('common.all'), value: 'all' },
	...albumOptions.value.map((album) => ({ label: albumLabel(album), value: album })),
]);

const sortOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('sort.recent'), value: 'recent-desc' },
	{ label: t('sort.titleAsc'), value: 'title-asc' },
	{ label: t('sort.titleDesc'), value: 'title-desc' },
	{ label: t('sort.artistAsc'), value: 'artist-asc' },
	{ label: t('sort.artistDesc'), value: 'artist-desc' },
	{ label: t('sort.albumAsc'), value: 'album-asc' },
	{ label: t('sort.durationShort'), value: 'duration-asc' },
	{ label: t('sort.durationLong'), value: 'duration-desc' },
]);

const favoriteLabel = (path: string) =>
	playerStore.isFavoritePath(path) ? t('common.removeFavorite') : t('common.addFavorite');

const toggleFavorite = (path: string) => {
	playerStore.toggleFavoritePath(path);
};

onMounted(async () => {
	await syncCachedTracksToDatabase();
	await loadLibrary();
	await playerStore.ensureMetadataForFavorites();
});

watch(searchQuery, () => {
	if (searchDebounceTimer !== null) {
		window.clearTimeout(searchDebounceTimer);
	}

	searchDebounceTimer = window.setTimeout(() => {
		void runFtsSearch();
	}, 180);
});

onUnmounted(() => {
	if (searchDebounceTimer !== null) {
		window.clearTimeout(searchDebounceTimer);
		searchDebounceTimer = null;
	}
});

// Con una sola pista, una única cadena decía «1 pistas visibles de 1 totales».
// Las dos mitades concuerdan por separado, y como no se pueden mostrar cinco
// pistas de un total de una, sólo tres de las cuatro combinaciones existen.
const visibleCountLabel = computed(() => {
	const visible = sortedTracks.value.length;
	const total = librarySourceTracks.value.length;
	const key =
		visible === 1
			? total === 1
				? 'home.visibleCountOneOfOne'
				: 'home.visibleCountOneOfOther'
			: 'home.visibleCountOther';

	return t(key).replace('{0}', String(visible)).replace('{1}', String(total));
});
</script>

<template>
	<!-- `@container`: los filtros se ponen en fila cuando la vista tiene lugar,
	     mirando su propio ancho y no el de la pantalla (antes `lg:`, 1024 de
	     ventana; 44 rem de vista es el mismo corte con la barra al costado). -->
	<section class="@container flex h-full flex-col gap-4 overflow-hidden p-4">
		<header class="shrink-0">
		<Panel class="gap-4">
			<PageHeader :eyebrow="t('home.eyebrow')" :title="t('home.title')" as="h2">
				<template #actions>
					<span class="text-xs text-tx-muted">{{ visibleCountLabel }}</span>
				</template>
			</PageHeader>

			<div class="grid gap-3 @min-[44rem]:grid-cols-[1.4fr_0.8fr_0.8fr_0.8fr]">
				<FormGroup :label="t('common.search')" variant="eyebrow">
					<SearchField
						v-model="searchQuery"
						:label="t('common.search')"
						:placeholder="t('home.searchPlaceholder')"
					/>
				</FormGroup>

				<!-- Los tres filtros, como antes, sólo con la vista ancha. -->
				<div class="hidden min-w-0 @min-[44rem]:block">
					<FormGroup v-slot="{ id }" :label="t('common.artist')" variant="eyebrow">
						<SelectField v-bind="{ id }" v-model="artistFilter" :options="artistSelectOptions" />
					</FormGroup>
				</div>

				<div class="hidden min-w-0 @min-[44rem]:block">
					<FormGroup v-slot="{ id }" :label="t('common.album')" variant="eyebrow">
						<SelectField v-bind="{ id }" v-model="albumFilter" :options="albumSelectOptions" />
					</FormGroup>
				</div>

				<div class="hidden min-w-0 @min-[44rem]:block">
					<FormGroup v-slot="{ id }" :label="t('common.sortBy')" variant="eyebrow">
						<SelectField v-bind="{ id }" v-model="sortBy" :options="sortOptions" />
					</FormGroup>
				</div>

				<div class="col-span-full flex items-end gap-2">
					<ActionButton
						:label="t('home.shuffle')"
						:title="t('home.shuffleHint')"
						icon="media-playlist-shuffle"
						variant="secondary"
						:disabled="sortedTracks.length === 0"
						@click="playRandomFiltered"
					/>
				</div>
			</div>
		</Panel>
		</header>

		<LoadingState v-if="isLoading" :label="t('home.loading')" bordered />

		<AlertMessage v-else-if="errorMessage" tone="error" icon="dialog-error">
			{{ errorMessage }}
		</AlertMessage>

		<EmptyState v-else-if="sortedTracks.length === 0" :title="t('home.noResults')" bordered />

		<!-- El menú es uno para toda la lista; cada fila dice cuál es la suya
		     con `data-track-path`. Así el `RecycleScroller` puede reciclar las
		     filas sin crear y destruir un menú por cada una. -->
		<Panel v-else padding="none" class="min-h-0 flex-1 overflow-hidden">
			<div ref="list" class="h-full min-h-0" @contextmenu="onTrackContextMenu">
				<RecycleScroller
					:items="sortedTracks"
					:key-field="'path'"
					:item-size="ROW_HEIGHT"
					class="h-full overflow-y-auto p-3"
					v-slot="{ item: track }"
				>
					<!-- La fila con el alto fijo de siempre: el desplazador la coloca
					     cada `ROW_HEIGHT` sin medirla. El envoltorio lleva la marca que
					     busca el menú del clic derecho; adentro, `ListRow`. -->
					<div :data-track-path="track.path" class="mb-2 h-[84px]">
					<ListRow
						class="h-full border border-ui-line-weak"
						:class="{
							'track-playing': track.path === playerStore.currentPath,
							'track-playing--active':
								track.path === playerStore.currentPath && playerStore.isPlaying,
						}"
					>
						<span class="flex min-w-0 items-center gap-2">
							<span class="truncate text-label-m">{{ track.title }}</span>
							<Badge class="tabular-nums">{{ formatDuration(track.duration_seconds) }}</Badge>
						</span>
						<span class="truncate text-xs text-tx-muted">{{ artistLabel(track.artist) }} • {{ albumLabel(track.album) }}</span>
						<span class="truncate text-[11px] text-tx-muted">{{ track.path }}</span>

						<template #trailing>
							<ActionButton
								:label="labeledActions ? t('common.play') : ''"
								:icon-alt="t('common.play')"
								:title="t('common.play')"
								icon="media-playback-start"
								@click="playTrack(track.path)"
							/>
							<ActionButton
								:label="labeledActions ? (playerStore.isFavoritePath(track.path) ? t('common.removeFavorite') : t('common.favorite')) : ''"
								:icon-alt="favoriteLabel(track.path)"
								:title="favoriteLabel(track.path)"
								:icon="playerStore.isFavoritePath(track.path) ? 'remove' : 'new-star'"
								variant="secondary"
								@click="toggleFavorite(track.path)"
							/>
						</template>
					</ListRow>
					</div>
				</RecycleScroller>
			</div>
		</Panel>
	</section>
</template>
