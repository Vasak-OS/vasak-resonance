<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref, watch } from 'vue';
import { fetchLyrics, type LyricsLine, type TrackLyricsPayload } from '@/services/player.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();

const loading = ref(false);
const error = ref('');
const lyrics = ref<TrackLyricsPayload | null>(null);
const plainLines = ref<string[]>([]);

const syncedLines = computed<LyricsLine[]>(() => lyrics.value?.lines ?? []);
const isSynced = computed(() => syncedLines.value.length > 0);

const currentLineIndex = computed(() => {
	if (!isSynced.value) {
		return -1;
	}

	const positionMs = Math.floor(playerStore.positionSeconds * 1000);
	const lines = syncedLines.value;
	let left = 0;
	let right = lines.length - 1;
	let best = -1;

	while (left <= right) {
		const mid = Math.floor((left + right) / 2);
		if (lines[mid].time_ms <= positionMs) {
			best = mid;
			left = mid + 1;
		} else {
			right = mid - 1;
		}
	}

	return best;
});

const loadLyricsForCurrentTrack = async () => {
	const track = playerStore.currentTrack;
	if (!track) {
		lyrics.value = null;
		plainLines.value = [];
		error.value = '';
		return;
	}

	loading.value = true;
	error.value = '';
	try {
		const payload = await fetchLyrics({
			trackName: track.title || '',
			artistName: track.artist || '',
			albumName: track.album || '',
			durationSeconds: track.duration_seconds || 0,
		});
		lyrics.value = payload;
		plainLines.value = (payload.plain_lyrics || '')
			.split('\n')
			.map((line) => line.trim())
			.filter((line) => line.length > 0);
	} catch (lyricsError: unknown) {
		lyrics.value = null;
		plainLines.value = [];
		console.error('[LyricsView] fetchLyrics error:', lyricsError);
		error.value = t('lyrics.loadError').replace('{0}', String(lyricsError));
	} finally {
		loading.value = false;
	}
};

watch(
	() => playerStore.currentTrack?.path,
	() => {
		void loadLyricsForCurrentTrack();
	},
	{ immediate: true }
);
</script>

<template>
	<section class="rounded-corner border border-primary/20 bg-ui-surface/40 px-3 py-2">
		<div class="mb-1 flex items-center justify-between gap-2">
			<p class="text-[10px] font-semibold uppercase tracking-[0.14em] text-primary">{{ t('lyrics.title') }}</p>
			<p v-if="loading" class="text-[10px] text-tx-muted">{{ t('lyrics.searching') }}</p>
		</div>

		<div class="max-h-36 overflow-y-auto pr-1 text-sm leading-relaxed">
			<p v-if="error" class="text-xs text-tx-muted">{{ error }}</p>
			<p v-else-if="!lyrics && !loading" class="text-xs text-tx-muted">{{ t('lyrics.unavailable') }}</p>

			<template v-else-if="isSynced">
				<Transition name="lyrics-line" mode="out-in">
					<p
						v-if="currentLineIndex >= 0"
						:key="currentLineIndex"
						class="text-center text-secondary font-semibold"
					>
						{{ syncedLines[currentLineIndex]?.text || '...' }}
					</p>
					<p v-else :key="-1" class="text-center text-tx-muted text-xs">
						{{ t('lyrics.waitingSync') }}
					</p>
				</Transition>
			</template>

			<template v-else>
				<p v-for="(line, index) in plainLines" :key="`plain-${index}`" class="text-tx-muted">
					{{ line }}
				</p>
			</template>
		</div>
	</section>
</template>

<style scoped>
.lyrics-line-enter-active,
.lyrics-line-leave-active {
	transition: all 300ms ease;
}

.lyrics-line-enter-from {
	opacity: 0;
	transform: translateY(8px);
}

.lyrics-line-leave-to {
	opacity: 0;
	transform: translateY(-8px);
}
</style>
