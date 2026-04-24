<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import PlayerControls from '@/components/player/PlayerControls.vue';
import { usePlayerStore } from '@/stores/player';
import { listLibraryTracks, type LibraryTrack } from '@/services/player.service';

const playerStore = usePlayerStore();
const libraryTracks = ref<LibraryTrack[]>([]);
const isLoading = ref(false);
const errorMessage = ref('');
const searchQuery = ref('');
const artistFilter = ref('all');
const albumFilter = ref('all');
const sortBy = ref('recent-desc');

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

const loadLibrary = async () => {
	isLoading.value = true;
	errorMessage.value = '';
	try {
		libraryTracks.value = await listLibraryTracks();
	} catch (error) {
		errorMessage.value = `No se pudo cargar la biblioteca: ${String(error)}`;
	} finally {
		isLoading.value = false;
	}
};

const artistOptions = computed(() => {
	const values = new Set(
		libraryTracks.value
			.map((track) => track.artist?.trim())
			.filter((value): value is string => Boolean(value))
	);
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

const albumOptions = computed(() => {
	const values = new Set(
		libraryTracks.value
			.map((track) => track.album?.trim())
			.filter((value): value is string => Boolean(value))
	);
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

const sortedTracks = computed(() => {
	const query = normalize(searchQuery.value);
	const filtered = libraryTracks.value.filter((track) => {
		if (artistFilter.value !== 'all' && track.artist !== artistFilter.value) {
			return false;
		}

		if (albumFilter.value !== 'all' && track.album !== albumFilter.value) {
			return false;
		}

		if (!query) {
			return true;
		}

		return [track.title, track.artist, track.album, track.path]
			.map((value) => normalize(value))
			.some((value) => value.includes(query));
	});

	return filtered.sort((left, right) => {
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
			case 'recent-desc':
			default:
				return right.created_at.localeCompare(left.created_at) || left.title.localeCompare(right.title);
		}
	});
});

const playTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

const toggleFavorite = (path: string) => {
	playerStore.toggleFavoritePath(path);
};

onMounted(async () => {
	await loadLibrary();
	await playerStore.ensureMetadataForFavorites();
});
</script>

<template>
	<section class="flex h-full flex-col gap-4 overflow-hidden p-4">
		<header class="space-y-4 rounded-corner border border-ui-border bg-ui-bg/80 p-4">
			<div class="flex flex-wrap items-end justify-between gap-4">
				<div>
					<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Biblioteca</p>
					<h2 class="text-lg font-semibold text-tx-main">Todas las canciones de la base de datos</h2>
				</div>
				<div class="text-xs text-tx-muted">
					{{ sortedTracks.length }} pistas visibles de {{ libraryTracks.length }} totales
				</div>
			</div>

			<div class="grid gap-3 lg:grid-cols-[1.4fr_0.8fr_0.8fr_0.8fr]">
				<label class="grid gap-1 text-xs uppercase tracking-[0.14em] text-tx-muted">
					Buscar
					<input
						v-model="searchQuery"
						type="search"
						placeholder="Título, artista, album o ruta"
						class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main outline-none transition-colors duration-200 placeholder:text-tx-muted/70 focus:border-primary/50"
					/>
				</label>

				<label class="grid gap-1 text-xs uppercase tracking-[0.14em] text-tx-muted">
					Artista
					<select v-model="artistFilter" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
						<option value="all">Todos</option>
						<option v-for="artist in artistOptions" :key="artist" :value="artist">{{ artist }}</option>
					</select>
				</label>

				<label class="grid gap-1 text-xs uppercase tracking-[0.14em] text-tx-muted">
					Album
					<select v-model="albumFilter" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
						<option value="all">Todos</option>
						<option v-for="album in albumOptions" :key="album" :value="album">{{ album }}</option>
					</select>
				</label>

				<label class="grid gap-1 text-xs uppercase tracking-[0.14em] text-tx-muted">
					Ordenar
					<select v-model="sortBy" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main outline-none transition-colors duration-200 focus:border-primary/50">
						<option value="recent-desc">Recientes primero</option>
						<option value="title-asc">Título A-Z</option>
						<option value="title-desc">Título Z-A</option>
						<option value="artist-asc">Artista A-Z</option>
						<option value="artist-desc">Artista Z-A</option>
						<option value="album-asc">Album A-Z</option>
						<option value="duration-asc">Duración corta</option>
						<option value="duration-desc">Duración larga</option>
					</select>
				</label>
			</div>
		</header>

		<div v-if="isLoading" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			Cargando biblioteca...
		</div>

		<div v-else-if="errorMessage" class="rounded-corner border border-status-error/35 bg-status-error/10 p-4 text-sm text-status-error">
			{{ errorMessage }}
		</div>

		<div v-else-if="sortedTracks.length === 0" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			No hay resultados con esos filtros.
		</div>

		<div v-else class="min-h-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80">
			<div class="h-full overflow-y-auto p-3">
				<div class="grid gap-2">
					<article
						v-for="track in sortedTracks"
						:key="track.path"
						class="flex flex-wrap items-center gap-3 rounded-corner border border-ui-border bg-ui-surface/45 px-3 py-2.5 transition-colors duration-200 hover:border-primary/35 hover:bg-ui-surface/70"
					>
						<div class="min-w-0 flex-1">
							<div class="flex flex-wrap items-center gap-2">
								<p class="truncate text-sm font-medium text-tx-main">{{ track.title }}</p>
								<span class="rounded-full border border-ui-border bg-ui-bg/60 px-2 py-0.5 text-[10px] uppercase tracking-[0.14em] text-tx-muted">
									{{ formatDuration(track.duration_seconds) }}
								</span>
							</div>
							<p class="truncate text-xs text-tx-muted">{{ track.artist }} • {{ track.album }}</p>
							<p class="truncate text-[11px] text-tx-muted/80">{{ track.path }}</p>
						</div>

						<div class="flex items-center gap-2">
							<button
								type="button"
								class="rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90"
								@click="playTrack(track.path)"
							>
								Reproducir
							</button>
							<button
								type="button"
								class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
								@click="toggleFavorite(track.path)"
							>
								{{ playerStore.isFavoritePath(track.path) ? 'Quitar favorito' : 'Favorito' }}
							</button>
						</div>
					</article>
				</div>
			</div>
		</div>

		<PlayerControls class="shrink-0" />
	</section>
</template>
