<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	EmptyState,
	FormGroup,
	ListRow,
	PageHeader,
	SearchField,
	SectionHeading,
	TextInput,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import PlayerQueuePanel from '@/components/player/PlayerQueuePanel.vue';
import { useElementWidth } from '@/composables/useElementWidth';
import { formatSeconds } from '@/composables/useTimeFormat';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { type LibraryTrack, listLibraryTracks } from '@/services/player.service';
import {
	addTrackToPlaylist,
	createPlaylist,
	deletePlaylist,
	listPlaylists,
	listPlaylistTracks,
	type Playlist,
	type PlaylistTrack,
	removeTrackFromPlaylist,
} from '@/services/playlists.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();

const playlists = ref<Playlist[]>([]);
const selectedPlaylist = ref<Playlist | null>(null);
const playlistTracks = ref<PlaylistTrack[]>([]);
const libraryTracks = ref<LibraryTrack[]>([]);

const newPlaylistName = ref('');
const addSearch = ref('');
const busy = ref(false);
const error = ref('');

/**
 * Lo que mide una fila de la lista, contando el hueco de abajo: 54 de alto más
 * los 4 del `mb-1`. El scroller coloca a partir de este número, así que si se
 * separa de lo que dibuja el CSS las filas se pisan o dejan huecos.
 */
const ROW_HEIGHT = 58;

/** Hasta dónde crece la caja de la lista antes de desplazarse ella sola. */
const MAX_LIST_HEIGHT = 480;

const listHeight = computed(() =>
	Math.min(playlistTracks.value.length * ROW_HEIGHT, MAX_LIST_HEIGHT)
);

/**
 * Una columna por vez cuando la vista es angosta: la lista de listas, y al
 * elegir una, su detalle con «Volver». Es el mismo corte que separa las dos
 * columnas (44 rem de vista, el `lg:` de antes con la barra al costado).
 */
const TWO_COLUMNS_MIN_WIDTH = 704;
const view = ref<HTMLElement | null>(null);
const viewWidth = useElementWidth(view);
const singleColumn = computed(() => viewWidth.value > 0 && viewWidth.value < TWO_COLUMNS_MIN_WIDTH);
/** En una columna: si se está mirando el detalle de la lista elegida. */
const detailOpen = ref(false);

const showPlaylists = computed(() => !singleColumn.value || !detailOpen.value);
const showDetail = computed(() => !singleColumn.value || detailOpen.value);

const addableTracks = computed(() => {
	const alreadyIn = new Set(playlistTracks.value.map((track) => track.track_id));
	const needle = addSearch.value.trim().toLowerCase();

	return libraryTracks.value
		.filter((track) => !alreadyIn.has(track.id))
		.filter((track) =>
			needle ? `${track.title} ${track.artist} ${track.album}`.toLowerCase().includes(needle) : true
		)
		.slice(0, 50);
});

const totalDuration = computed(() =>
	playlistTracks.value.reduce((sum, track) => sum + (track.duration_seconds || 0), 0)
);

// Una lista con una sola canción decía «1 canciones».
const trackSummaryLabel = computed(() => {
	const count = playlistTracks.value.length;

	return t(count === 1 ? 'playlists.trackSummaryOne' : 'playlists.trackSummaryOther')
		.replace('{0}', String(count))
		.replace('{1}', () => formatSeconds(totalDuration.value));
});

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
		detailOpen.value = true;
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

/**
 * Reproducir algo de la cola lo saca de la cola, igual que cuando avanza sola:
 * si se quedara, la canción volvería a sonar más tarde sin motivo.
 *
 * Se recibe el identificador de la entrada y no su posición, porque la cola
 * puede haber avanzado desde que se abrió el menú.
 */
const playFromQueue = async (id: string) => {
	const entry = playerStore.getQueueEntry(id);
	if (!entry) {
		return;
	}

	playerStore.removeQueueItem(id);
	await playerStore.playDropped(entry.path);
};

onMounted(() =>
	run(async () => {
		// Both lists are needed before anything can be added to a playlist.
		[libraryTracks.value] = await Promise.all([listLibraryTracks(), loadPlaylists()]);
	})
);
</script>

<template>
	<!-- `@container`: las dos columnas van juntas cuando la vista tiene lugar
	     (antes `lg:`, 1024 de ventana; 44 rem de vista con la barra al
	     costado). Más angosta, una por vez. -->
	<section ref="view" class="@container h-full overflow-y-auto p-4">
		<PageHeader class="mb-4" :eyebrow="t('playlists.eyebrow')" :title="t('playlists.title')" as="h2" />

		<AlertMessage v-if="error" class="mb-4" tone="error" icon="auto">
			{{ error }}
		</AlertMessage>

		<div class="grid gap-4 @min-[44rem]:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
			<!-- The lists themselves -->
			<div v-show="showPlaylists" class="flex min-w-0 flex-col gap-3">
				<form class="flex min-w-0 gap-2" @submit.prevent="submitNewPlaylist">
					<TextInput
						v-model="newPlaylistName"
						class="flex-1"
						:placeholder="t('playlists.namePlaceholder')"
						:ariaLabel="t('playlists.namePlaceholder')"
					/>
					<ActionButton
						type="submit"
						:label="t('playlists.create')"
						:disabled="busy || !newPlaylistName.trim()"
					/>
				</form>

				<EmptyState v-if="playlists.length === 0" :title="t('playlists.empty')" bordered />

				<ListRow
					v-for="playlist in playlists"
					:key="playlist.id"
					role="button"
					class="border border-ui-line"
					:title="playlist.name"
					truncate
					:selected="selectedPlaylist?.id === playlist.id"
					@click="selectPlaylist(playlist)"
				>
					<template #trailing>
						<ActionButton
							label=""
							:icon-alt="t('playlists.deleteList')"
							:title="t('playlists.deleteList')"
							icon="window-close-symbolic"
							variant="ghost"
							size="sm"
							stop-propagation
							@click="removePlaylist(playlist)"
						/>
					</template>
				</ListRow>
			</div>

			<!-- The open list -->
			<div v-if="selectedPlaylist" v-show="showDetail" class="flex min-w-0 flex-col gap-4">
				<ActionButton
					v-if="singleColumn"
					class="self-start"
					:label="t('artists.back')"
					icon="go-previous-symbolic"
					variant="secondary"
					@click="detailOpen = false"
				/>
				<div class="flex min-w-0 flex-wrap items-center gap-3">
					<div class="min-w-0 flex-1">
						<h3 class="truncate text-base font-semibold text-tx-main">
							{{ selectedPlaylist.name }}
						</h3>
						<p class="text-xs text-tx-muted">
							{{ trackSummaryLabel }}
						</p>
					</div>
					<ActionButton
						:label="t('common.play')"
						:disabled="busy || playlistTracks.length === 0"
						@click="playPlaylist"
					/>
					<ActionButton
						:label="t('playlists.enqueue')"
						variant="secondary"
						:disabled="busy || playlistTracks.length === 0"
						@click="enqueuePlaylist"
					/>
				</div>

				<EmptyState v-if="playlistTracks.length === 0" :title="t('playlists.emptyList')" bordered />

				<!--
					La lista se virtualiza con alto propio y no en «modo página»:
					en la 2 de `vue-virtual-scroller` ese modo mira el
					desplazamiento de la ventana y no busca el contenedor que
					desplaza de verdad —que acá es la sección—, así que colocaría
					las filas en el lugar equivocado. Buscar el contenedor llegó
					en la 3.0.4, y a la 3 no subimos: su `DynamicScroller` es
					cerca del doble de lento en WebKitGTK y `AlbumsView` lo usa.

					El alto sale de cuántas pistas hay, con tope: una lista de
					tres no deja un hueco vacío, y una de mil no empuja el resto
					de la vista fuera de la pantalla.
				-->
				<div
					v-else
					class="overflow-hidden"
					:style="{ height: `${listHeight}px` }"
					@contextmenu="onTrackContextMenu"
				>
					<RecycleScroller
						:items="playlistTracks"
						key-field="track_id"
						:item-size="ROW_HEIGHT"
						class="h-full overflow-y-auto"
						v-slot="{ item: track, index }"
					>
						<div :data-track-path="track.path" class="mb-1 h-[54px]">
							<ListRow
								class="h-full border border-ui-line-weak"
								:title="track.title"
								:description="track.artist"
								:meta="formatSeconds(track.duration_seconds)"
								truncate
							>
								<template #leading>
									<span class="w-6 shrink-0 text-right text-xs text-tx-muted tabular-nums">{{ index + 1 }}</span>
								</template>
								<template #trailing>
									<ActionButton
										label=""
										:icon-alt="t('playlists.removeFromList')"
										:title="t('playlists.removeFromList')"
										icon="window-close-symbolic"
										variant="ghost"
										size="sm"
										@click="removeTrack(track)"
									/>
								</template>
							</ListRow>
						</div>
					</RecycleScroller>
				</div>

				<!-- Adding from the library -->
				<div class="flex flex-col gap-2 border-t border-ui-line-weak pt-4">
					<FormGroup :label="t('playlists.addFromLibrary')" variant="eyebrow">
						<SearchField
							v-model="addSearch"
							:label="t('playlists.addFromLibrary')"
							:placeholder="t('playlists.addSearchPlaceholder')"
						/>
					</FormGroup>

					<p v-if="libraryTracks.length === 0" class="text-sm text-tx-muted">
						{{ t('playlists.libraryEmpty') }}
					</p>
					<p v-else-if="addableTracks.length === 0" class="text-sm text-tx-muted">
						{{ t('playlists.noMatches') }}
					</p>

					<ListRow
						v-for="track in addableTracks"
						:key="track.id"
						role="button"
						class="border border-ui-line-weak"
						icon="list-add-symbolic"
						icon-type="symbol"
						:title="track.title"
						:description="`${track.artist} · ${track.album}`"
						truncate
						@click="addTrack(track)"
					/>
				</div>
			</div>
		</div>

		<!-- The play queue used to be the whole of this screen; it belongs here,
		     but as what it is rather than under the "Playlists" name. -->
		<div v-if="playerStore.queue.length > 0" class="mt-8 border-t border-ui-line-weak pt-4">
			<SectionHeading class="mb-3" :title="t('queue.heading')" />
			<PlayerQueuePanel
				:queue-items="playerStore.queueEntries"
				@clear="playerStore.clearQueue"
				@play="playFromQueue"
				@remove="playerStore.removeQueueItem"
				@reorder="playerStore.moveQueueItem"
			/>
		</div>
	</section>
</template>
