<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import LabeledField from '@/components/layout/LabeledField.vue';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { usePlayerStore } from '@/stores/player';
import { agruparPorArtista } from '@/tools/artistas';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();
const searchQuery = ref('');
/** El artista que se está mirando, o `null` para la lista. */
const artistaAbierto = ref<string | null>(null);

const normalizar = (valor: string) => valor.trim().toLowerCase();

const artistas = computed(() =>
	agruparPorArtista(playerStore.trackCacheList, t('common.unknownTrack'))
);

const artistasFiltrados = computed(() => {
	const consulta = normalizar(searchQuery.value);
	if (!consulta) {
		return artistas.value;
	}
	// Se busca por el nombre traducido además del crudo: quien ve «Artista
	// desconocido» en pantalla espera encontrarlo escribiendo eso.
	return artistas.value.filter(
		(artista) =>
			normalizar(artista.nombre).includes(consulta) ||
			normalizar(artistLabel(artista.nombre)).includes(consulta)
	);
});

const artista = computed(
	() => artistas.value.find((candidato) => candidato.clave === artistaAbierto.value) ?? null
);

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const abrir = (clave: string) => {
	artistaAbierto.value = clave;
};

const volver = () => {
	artistaAbierto.value = null;
};

const onPlayTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

const onQueue = (paths: string[], nombre: string) => {
	playerStore.showGlobalBadge(t('artists.queued').replace('{0}', () => nombre));
	playerStore.enqueuePaths(paths);
};

const onPlay = async (paths: string[], nombre: string) => {
	playerStore.showGlobalBadge(t('artists.playing').replace('{0}', () => nombre));
	await playerStore.playAlbum(paths);
};

/** Todo lo del artista, en el orden en que se muestra: disco por disco. */
const temasDelArtista = (clave: string): string[] => {
	const encontrado = artistas.value.find((candidato) => candidato.clave === clave);
	if (!encontrado) {
		return [];
	}
	return encontrado.discos.flatMap((disco) => disco.tracks.map((pista) => pista.path));
};
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">{{ t('artists.eyebrow') }}</p>
			<h2 class="text-lg font-semibold text-tx-main">{{ t('artists.title') }}</h2>
		</div>

		<!-- La lista de artistas -->
		<template v-if="!artista">
			<div class="mb-4">
				<LabeledField :label="t('common.search')">
					<input
						v-model="searchQuery"
						type="search"
						:placeholder="t('artists.searchPlaceholder')"
						class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 placeholder:text-tx-muted/70 focus:border-primary/50"
					/>
				</LabeledField>
			</div>

			<EmptyState v-if="artistasFiltrados.length === 0" :title="t('artists.empty')" bordered />

			<div v-else class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				<article
					v-for="candidato in artistasFiltrados"
					:key="candidato.clave"
					class="rounded-corner border border-ui-border bg-ui-bg/80 p-4"
				>
					<button
						type="button"
						class="block w-full text-left"
						:aria-label="t('artists.open').replace('{0}', () => artistLabel(candidato.nombre))"
						@click="abrir(candidato.clave)"
					>
						<div class="mb-3 flex h-44 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
							<img v-if="candidato.tapa" :src="candidato.tapa" :alt="artistLabel(candidato.nombre)" class="h-full w-full object-cover" />
							<div v-else class="text-sm font-semibold uppercase tracking-[0.16em] text-tx-muted">{{ t('common.noCover') }}</div>
						</div>
						<p class="truncate text-base font-semibold text-tx-main">{{ artistLabel(candidato.nombre) }}</p>
						<p class="mt-2 text-xs uppercase tracking-[0.12em] text-primary">
							{{ t(candidato.discos.length === 1 ? 'artists.albumCountOne' : 'artists.albumCountOther')
								.replace('{0}', String(candidato.discos.length)) }}
							·
							{{ t(candidato.temas.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
								.replace('{0}', String(candidato.temas.length)) }}
						</p>
					</button>

					<div class="mt-3 grid grid-cols-2 gap-2">
						<button
							type="button"
							class="inline-flex w-full items-center justify-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90"
							:title="t('artists.playAll')"
							:aria-label="t('artists.playAll')"
							@click="onPlay(temasDelArtista(candidato.clave), artistLabel(candidato.nombre))"
						>
							<ThemeIcon name="media-playback-start" type="symbol" :size="16" />
							{{ t('artists.playAll') }}
						</button>
						<button
							type="button"
							class="inline-flex w-full items-center justify-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
							:title="t('artists.queueAll')"
							:aria-label="t('artists.queueAll')"
							@click="onQueue(temasDelArtista(candidato.clave), artistLabel(candidato.nombre))"
						>
							<ThemeIcon name="media-track-add-amarok" type="symbol" :size="16" />
							{{ t('artists.queueAll') }}
						</button>
					</div>
				</article>
			</div>
		</template>

		<!-- Un artista, con sus discos -->
		<template v-else>
			<div class="mb-4 flex flex-wrap items-center gap-3">
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					@click="volver"
				>
					<ThemeIcon name="go-previous-symbolic" type="symbol" :size="16" />
					{{ t('artists.back') }}
				</button>
				<h3 class="min-w-0 flex-1 truncate text-lg font-semibold text-tx-main">
					{{ artistLabel(artista.nombre) }}
				</h3>
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90"
					@click="onPlay(temasDelArtista(artista.clave), artistLabel(artista.nombre))"
				>
					<ThemeIcon name="media-playback-start" type="symbol" :size="16" />
					{{ t('artists.playAll') }}
				</button>
			</div>

			<!-- Un solo menú para todo; cada pista dice cuál es la suya. -->
			<div class="grid gap-4" @contextmenu="onTrackContextMenu">
				<article
					v-for="disco in artista.discos"
					:key="disco.key"
					class="rounded-corner border border-ui-border bg-ui-bg/80 p-4"
				>
					<div class="mb-3 flex flex-wrap items-center gap-3">
						<div class="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
							<img v-if="disco.cover" :src="disco.cover" :alt="albumLabel(disco.album)" class="h-full w-full object-cover" />
						</div>
						<div class="min-w-0 flex-1">
							<p class="truncate text-base font-semibold text-tx-main">{{ albumLabel(disco.album) }}</p>
							<p class="text-xs uppercase tracking-[0.12em] text-primary">
								{{ t(disco.tracks.length === 1 ? 'albums.trackCountOne' : 'albums.trackCountOther')
									.replace('{0}', String(disco.tracks.length)) }}
							</p>
						</div>
						<button
							type="button"
							class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
							:title="t('albums.queueAlbum')"
							:aria-label="t('albums.queueAlbum')"
							@click="onQueue(disco.tracks.map((pista) => pista.path), albumLabel(disco.album))"
						>
							<ThemeIcon name="media-track-add-amarok" type="symbol" :size="16" />
							{{ t('albums.queueAlbum') }}
						</button>
					</div>

					<ul class="grid gap-1.5">
						<li
							v-for="pista in disco.tracks"
							:key="pista.path"
							:data-track-path="pista.path"
							class="flex min-w-0 items-center gap-2 rounded-corner border border-transparent px-2 py-1 hover:border-ui-border hover:bg-ui-surface/45"
						>
							<span v-if="pista.track_no > 0" class="w-6 shrink-0 text-right text-xs tabular-nums text-tx-muted">
								{{ pista.track_no }}
							</span>
							<p class="min-w-0 flex-1 truncate text-xs text-tx-muted">
								{{ pista.title || extractTrackName(pista.path) }}
							</p>
							<button
								type="button"
								class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-corner border border-primary/45 bg-primary/10 text-[11px] font-medium text-primary transition-colors duration-200 hover:bg-primary/20"
								:title="t('common.play')"
								:aria-label="t('common.play')"
								@click="onPlayTrack(pista.path)"
							>
								<ThemeIcon name="media-playback-start" type="symbol" :size="14" />
								<span class="sr-only">{{ t('common.play') }}</span>
							</button>
						</li>
					</ul>
				</article>
			</div>
		</template>
	</section>
</template>
