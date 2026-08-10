<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import PlayerQueuePanel from '@/components/player/PlayerQueuePanel.vue';
import { formatSeconds } from '@/composables/useTimeFormat';
import { listLibraryTracks, type LibraryTrack } from '@/services/player.service';
import {
	addTrackToPlaylist,
	createPlaylist,
	deletePlaylist,
	listPlaylistTracks,
	listPlaylists,
	type Playlist,
	type PlaylistTrack,
	removeTrackFromPlaylist,
} from '@/services/playlists.service';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const playlists = ref<Playlist[]>([]);
const selectedPlaylist = ref<Playlist | null>(null);
const playlistTracks = ref<PlaylistTrack[]>([]);
const libraryTracks = ref<LibraryTrack[]>([]);

const newPlaylistName = ref('');
const addSearch = ref('');
const busy = ref(false);
const error = ref('');

/** Library tracks not already in the open playlist, filtered by the search box. */
const addableTracks = computed(() => {
	const alreadyIn = new Set(playlistTracks.value.map((track) => track.track_id));
	const needle = addSearch.value.trim().toLowerCase();

	return libraryTracks.value
		.filter((track) => !alreadyIn.has(track.id))
		.filter((track) =>
			needle
				? `${track.title} ${track.artist} ${track.album}`.toLowerCase().includes(needle)
				: true
		)
		.slice(0, 50);
});

const totalDuration = computed(() =>
	playlistTracks.value.reduce((sum, track) => sum + (track.duration_seconds || 0), 0)
);

const run = async (action: () => Promise<void>) => {
	busy.value = true;
	error.value = '';
	try {
		await action();
	} catch (err) {
		error.value = String(err);
	} finally {
		busy.value = false;
	}
};

const loadPlaylists = async () => {
	playlists.value = await listPlaylists();

	// Keep the open playlist selected across reloads; fall back to the first.
	const stillThere = playlists.value.find((item) => item.id === selectedPlaylist.value?.id);
	selectedPlaylist.value = stillThere ?? playlists.value[0] ?? null;
	await loadTracks();
};

const loadTracks = async () => {
	playlistTracks.value = selectedPlaylist.value
		? await listPlaylistTracks(selectedPlaylist.value.id)
		: [];
};

const selectPlaylist = (playlist: Playlist) =>
	run(async () => {
		selectedPlaylist.value = playlist;
		await loadTracks();
	});

const submitNewPlaylist = () =>
	run(async () => {
		const name = newPlaylistName.value.trim();
		if (!name) return;

		selectedPlaylist.value = await createPlaylist(name);
		newPlaylistName.value = '';
		await loadPlaylists();
	});

const removePlaylist = (playlist: Playlist) =>
	run(async () => {
		await deletePlaylist(playlist.id);
		if (selectedPlaylist.value?.id === playlist.id) {
			selectedPlaylist.value = null;
		}
		await loadPlaylists();
	});

const addTrack = (track: LibraryTrack) =>
	run(async () => {
		if (!selectedPlaylist.value) return;
		await addTrackToPlaylist(selectedPlaylist.value.id, track.id);
		await loadTracks();
	});

const removeTrack = (track: PlaylistTrack) =>
	run(async () => {
		await removeTrackFromPlaylist(track.playlist_id, track.track_id);
		await loadTracks();
	});

const playPlaylist = () =>
	run(async () => {
		if (playlistTracks.value.length === 0) return;
		await playerStore.playAlbum(playlistTracks.value.map((track) => track.path));
	});

const enqueuePlaylist = () =>
	run(async () => {
		playerStore.enqueuePaths(playlistTracks.value.map((track) => track.path));
	});

onMounted(() =>
	run(async () => {
		// Both lists are needed before anything can be added to a playlist.
		[libraryTracks.value] = await Promise.all([listLibraryTracks(), loadPlaylists()]);
	})
);
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Playlists</p>
			<h2 class="text-lg font-semibold text-tx-main">Tus listas de reproducción</h2>
		</div>

		<p
			v-if="error"
			class="mb-4 rounded-corner border border-status-error/30 bg-status-error/10 p-2 text-sm text-status-error"
		>
			{{ error }}
		</p>

		<div class="grid gap-4 lg:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
			<!-- The lists themselves -->
			<div class="flex flex-col gap-3">
				<form class="flex gap-2" @submit.prevent="submitNewPlaylist">
					<input
						v-model="newPlaylistName"
						type="text"
						placeholder="Nombre de la lista"
						class="min-w-0 flex-1 rounded-corner border border-ui-border bg-ui-surface/40 p-2 text-sm text-tx-main focus:outline-none focus:ring-2 focus:ring-primary"
					/>
					<button
						type="submit"
						:disabled="busy || !newPlaylistName.trim()"
						class="rounded-corner bg-primary px-3 py-2 text-sm font-semibold text-tx-on-primary disabled:opacity-50"
					>
						Crear
					</button>
				</form>

				<p
					v-if="playlists.length === 0"
					class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted"
				>
					Todavía no tenés ninguna lista. Creá una para guardar canciones y volver a
					escucharlas cuando quieras.
				</p>

				<div
					v-for="playlist in playlists"
					:key="playlist.id"
					class="flex items-center gap-2 rounded-corner border p-3 transition-colors"
					:class="
						selectedPlaylist?.id === playlist.id
							? 'border-primary bg-secondary/20'
							: 'border-ui-border hover:bg-ui-surface/50'
					"
				>
					<button
						type="button"
						class="min-w-0 flex-1 text-left"
						@click="selectPlaylist(playlist)"
					>
						<span class="block truncate font-semibold text-tx-main">{{ playlist.name }}</span>
					</button>
					<button
						type="button"
						title="Eliminar lista"
						aria-label="Eliminar lista"
						class="shrink-0 rounded-corner px-2 py-1 text-sm text-tx-muted hover:bg-status-error/15 hover:text-status-error"
						@click="removePlaylist(playlist)"
					>
						✕
					</button>
				</div>
			</div>

			<!-- The open list -->
			<div v-if="selectedPlaylist" class="flex flex-col gap-4">
				<div class="flex flex-wrap items-center gap-3">
					<div class="min-w-0 flex-1">
						<h3 class="truncate text-base font-semibold text-tx-main">
							{{ selectedPlaylist.name }}
						</h3>
						<p class="text-xs text-tx-muted">
							{{ playlistTracks.length }} canciones · {{ formatSeconds(totalDuration) }}
						</p>
					</div>
					<button
						type="button"
						:disabled="busy || playlistTracks.length === 0"
						class="rounded-corner bg-primary px-3 py-2 text-sm font-semibold text-tx-on-primary disabled:opacity-50"
						@click="playPlaylist"
					>
						Reproducir
					</button>
					<button
						type="button"
						:disabled="busy || playlistTracks.length === 0"
						class="rounded-corner border border-ui-border px-3 py-2 text-sm text-tx-main disabled:opacity-50 hover:bg-ui-surface/50"
						@click="enqueuePlaylist"
					>
						Agregar a la cola
					</button>
				</div>

				<div
					v-if="playlistTracks.length === 0"
					class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted"
				>
					Esta lista está vacía. Agregá canciones desde tu biblioteca, abajo.
				</div>

				<ol v-else class="flex flex-col gap-1">
					<li
						v-for="(track, index) in playlistTracks"
						:key="track.track_id"
						class="flex items-center gap-3 rounded-corner border border-ui-border/60 p-2"
					>
						<span class="w-6 shrink-0 text-right text-xs text-tx-muted">{{ index + 1 }}</span>
						<div class="min-w-0 flex-1">
							<span class="block truncate text-sm text-tx-main">{{ track.title }}</span>
							<span class="block truncate text-xs text-tx-muted">{{ track.artist }}</span>
						</div>
						<span class="shrink-0 text-xs tabular-nums text-tx-muted">
							{{ formatSeconds(track.duration_seconds) }}
						</span>
						<button
							type="button"
							title="Quitar de la lista"
							aria-label="Quitar de la lista"
							class="shrink-0 rounded-corner px-2 py-1 text-sm text-tx-muted hover:bg-status-error/15 hover:text-status-error"
							@click="removeTrack(track)"
						>
							✕
						</button>
					</li>
				</ol>

				<!-- Adding from the library -->
				<div class="flex flex-col gap-2 border-t border-ui-border pt-4">
					<label class="text-xs uppercase tracking-[0.16em] text-tx-muted" for="add-search">
						Agregar de tu biblioteca
					</label>
					<input
						id="add-search"
						v-model="addSearch"
						type="search"
						placeholder="Buscar por título, artista o álbum"
						class="rounded-corner border border-ui-border bg-ui-surface/40 p-2 text-sm text-tx-main focus:outline-none focus:ring-2 focus:ring-primary"
					/>

					<p v-if="libraryTracks.length === 0" class="text-sm text-tx-muted">
						Tu biblioteca está vacía. Escaneá tu carpeta de música primero.
					</p>
					<p v-else-if="addableTracks.length === 0" class="text-sm text-tx-muted">
						No hay canciones que coincidan.
					</p>

					<button
						v-for="track in addableTracks"
						:key="track.id"
						type="button"
						class="flex items-center gap-3 rounded-corner border border-ui-border/60 p-2 text-left hover:bg-ui-surface/50"
						@click="addTrack(track)"
					>
						<span class="shrink-0 text-tx-muted">+</span>
						<div class="min-w-0 flex-1">
							<span class="block truncate text-sm text-tx-main">{{ track.title }}</span>
							<span class="block truncate text-xs text-tx-muted">
								{{ track.artist }} · {{ track.album }}
							</span>
						</div>
					</button>
				</div>
			</div>
		</div>

		<!-- The play queue used to be the whole of this screen; it belongs here,
		     but as what it is rather than under the "Playlists" name. -->
		<div v-if="playerStore.queue.length > 0" class="mt-8 border-t border-ui-border pt-4">
			<p class="mb-3 text-xs uppercase tracking-[0.16em] text-tx-muted">Cola de reproducción</p>
			<PlayerQueuePanel
				:queue-items="playerStore.queue"
				@clear="playerStore.clearQueue"
				@remove="playerStore.removeQueueItem"
				@reorder="playerStore.moveQueueItem"
			/>
		</div>
	</section>
</template>
