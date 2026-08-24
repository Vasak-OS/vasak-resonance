<script setup lang="ts">
import { RecycleScroller } from 'vue-virtual-scroller';
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import LabeledField from '@/components/layout/LabeledField.vue';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import {
	type DroppedPlaybackTrack,
	type LibraryTrack,
	listLibraryTracks,
	saveLibraryTrack,
	searchLibraryTracks,
} from '@/services/player.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();
const libraryTracks = ref<LibraryTrack[]>([]);
const isLoading = ref(false);
const errorMessage = ref('');
const searchQuery = ref('');
const artistFilter = ref('all');
const albumFilter = ref('all');
const sortBy = ref('recent-desc');
const ftsSearchResults = ref<LibraryTrack[] | null>(null);
const playIcon = useReactiveIcon('media-playback-start');
const addFavoriteIcon = useReactiveIcon('new-star');
const removeIcon = useReactiveIcon('remove');
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
		errorMessage.value = t('home.libraryLoadError').replace('{0}', String(error));
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

const playRandomFiltered = async () => {
	const list = sortedTracks.value ?? [];
	if (list.length === 0) {
		playerStore.globalBadgeMessage = t('home.nothingToPlay');
		return;
	}

	// Crear una copia y mezclar aleatoriamente
	const shuffled = [...list].sort(() => Math.random() - 0.5);
	const firstTrack = shuffled[0];
	const restTracks = shuffled.slice(1);

	// Reproducir la primera canción
	await playerStore.playDropped(firstTrack.path);

	// Agregar el resto a la cola
	if (restTracks.length > 0) {
		playerStore.enqueuePaths(restTracks.map((t) => t.path));
	}
};

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
</script>

<template>
	<section class="flex h-full flex-col gap-4 overflow-hidden p-4">
		<header class="space-y-4 rounded-corner border border-ui-border bg-ui-bg/80 p-4">
			<div class="flex flex-wrap items-end justify-between gap-4">
				<div>
					<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">{{ t('home.eyebrow') }}</p>
					<h2 class="text-lg font-semibold text-tx-main">{{ t('home.title') }}</h2>
				</div>
				<div class="text-xs text-tx-muted">
					{{ t('home.visibleCount')
						.replace('{0}', String(sortedTracks.length))
						.replace('{1}', String(librarySourceTracks.length)) }}
				</div>
			</div>

			<div class="grid gap-3 lg:grid-cols-[1.4fr_0.8fr_0.8fr_0.8fr]">
				<LabeledField :label="t('common.search')">
					<input
						v-model="searchQuery"
						type="search"
						:placeholder="t('home.searchPlaceholder')"
						class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main outline-none transition-colors duration-200 placeholder:text-tx-muted/70 focus:border-primary/50"
					/>
				</LabeledField>

				<LabeledField :label="t('common.artist')" wrapperClass="relative hidden lg:block">
					<div class="relative">
						<select v-model="artistFilter" class="appearance-none rounded-corner border border-ui-border bg-ui-surface/80 px-3 py-2 pr-8 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
							<option value="all">{{ t('common.all') }}</option>
							<option v-for="artist in artistOptions" :key="artist" :value="artist">{{ artistLabel(artist) }}</option>
						</select>
						<svg class="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-tx-muted" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 111.06 1.06l-4.24 4.24a.75.75 0 01-1.06 0L5.21 8.29a.75.75 0 01.02-1.08z" clip-rule="evenodd" />
						</svg>
					</div>
				</LabeledField>

				<LabeledField :label="t('common.album')" wrapperClass="relative hidden lg:block">
					<div class="relative">
						<select v-model="albumFilter" class="appearance-none rounded-corner border border-ui-border bg-ui-surface/80 px-3 py-2 pr-8 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
							<option value="all">{{ t('common.all') }}</option>
							<option v-for="album in albumOptions" :key="album" :value="album">{{ albumLabel(album) }}</option>
						</select>
						<svg class="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-tx-muted" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 111.06 1.06l-4.24 4.24a.75.75 0 01-1.06 0L5.21 8.29a.75.75 0 01.02-1.08z" clip-rule="evenodd" />
						</svg>
					</div>
				</LabeledField>

				<LabeledField :label="t('common.sortBy')" wrapperClass="relative hidden lg:block">
					<div class="relative">
						<select v-model="sortBy" class="appearance-none rounded-corner border border-ui-border bg-ui-surface/80 px-3 py-2 pr-8 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
							<option value="recent-desc">{{ t('sort.recent') }}</option>
							<option value="title-asc">{{ t('sort.titleAsc') }}</option>
							<option value="title-desc">{{ t('sort.titleDesc') }}</option>
							<option value="artist-asc">{{ t('sort.artistAsc') }}</option>
							<option value="artist-desc">{{ t('sort.artistDesc') }}</option>
							<option value="album-asc">{{ t('sort.albumAsc') }}</option>
							<option value="duration-asc">{{ t('sort.durationShort') }}</option>
							<option value="duration-desc">{{ t('sort.durationLong') }}</option>
						</select>
						<svg class="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-tx-muted" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 10.94l3.71-3.71a.75.75 0 111.06 1.06l-4.24 4.24a.75.75 0 01-1.06 0L5.21 8.29a.75.75 0 01.02-1.08z" clip-rule="evenodd" />
						</svg>
					</div>
				</LabeledField>

				<div class="col-span-full flex items-end gap-2">
					<button
						@click="playRandomFiltered"
						:disabled="sortedTracks.length === 0"
						class="inline-flex items-center gap-2 rounded-corner border border-secondary/50 bg-secondary/15 px-4 py-2 text-sm font-medium text-secondary transition-colors duration-200 hover:enabled:border-secondary/80 hover:enabled:bg-secondary/25 disabled:opacity-50 disabled:cursor-not-allowed"
:title="t('home.shuffleHint')"
					>
						<svg class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
							<path d="M3 2a1 1 0 011 1v2.101a7 7 0 0110.821 3.394c.105.302.214.602.321.901l1.196-.605A1 1 0 0117 7V4a1 1 0 00-2 0v1.101A9 9 0 005 3H4a1 1 0 00-1 1zm14 12a1 1 0 01-1 1h-1.101a7 7 0 01-10.82-3.394c-.105-.302-.214-.602-.321-.901l-1.196.605A1 1 0 003 13v3a1 1 0 002 0v-1.101A9 9 0 0015 17h1a1 1 0 001-1z" />
						</svg>
						<span>{{ t('home.shuffle') }}</span>
					</button>
				</div>
			</div>
		</header>

		<div v-if="isLoading" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			{{ t('home.loading') }}
		</div>

		<div v-else-if="errorMessage" class="rounded-corner border border-status-error/35 bg-status-error/10 p-4 text-sm text-status-error">
			{{ errorMessage }}
		</div>

		<div v-else-if="sortedTracks.length === 0" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			{{ t('home.noResults') }}
		</div>

		<!-- El menú es uno para toda la lista; cada fila dice cuál es la suya
		     con `data-track-path`. Así el `RecycleScroller` puede reciclar las
		     filas sin crear y destruir un menú por cada una. -->
		<div
			v-else
			class="min-h-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80"
			@contextmenu="onTrackContextMenu"
		>
			<RecycleScroller
				:items="sortedTracks"
				:key-field="'path'"
				:item-size="92"
				class="h-full overflow-y-auto p-3"
				v-slot="{ item: track }"
			>
				<article
					:data-track-path="track.path"
					class="mb-2 flex h-[84px] items-center gap-3 rounded-corner border border-ui-border bg-ui-surface/45 px-3 py-2.5 transition-colors duration-200 hover:border-primary/35 hover:bg-ui-surface/70"
				>
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-2">
							<p class="truncate text-sm font-medium text-tx-main">{{ track.title }}</p>
							<span class="rounded-full border border-ui-border bg-ui-bg/60 px-2 py-0.5 text-[10px] uppercase tracking-[0.14em] text-tx-muted">
								{{ formatDuration(track.duration_seconds) }}
							</span>
						</div>
						<p class="truncate text-xs text-tx-muted">{{ artistLabel(track.artist) }} • {{ albumLabel(track.album) }}</p>
						<p class="truncate text-[11px] text-tx-muted/80">{{ track.path }}</p>
					</div>

					<div class="flex items-center gap-2">
						<button
							type="button"
							class="inline-flex items-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90"
							:title="t('common.play')"
							:aria-label="t('common.play')"
							@click="playTrack(track.path)"
						>
							<img v-if="playIcon" :src="playIcon" :alt="t('common.play')" class="h-4 w-4">
							{{ t('common.play') }}
						</button>
						<button
							type="button"
							class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
							:title="playerStore.isFavoritePath(track.path) ? t('common.removeFavorite') : t('common.addFavorite')"
							:aria-label="playerStore.isFavoritePath(track.path) ? t('common.removeFavorite') : t('common.addFavorite')"
							@click="toggleFavorite(track.path)"
						>
							<img
								v-if="playerStore.isFavoritePath(track.path) ? removeIcon : addFavoriteIcon"
								:src="playerStore.isFavoritePath(track.path) ? removeIcon : addFavoriteIcon"
								:alt="playerStore.isFavoritePath(track.path) ? t('common.removeFavorite') : t('common.addFavorite')"
								class="h-4 w-4"
							>
							{{ playerStore.isFavoritePath(track.path) ? t('common.removeFavorite') : t('common.favorite') }}
						</button>
					</div>
				</article>
			</RecycleScroller>
		</div>

	</section>
</template>
