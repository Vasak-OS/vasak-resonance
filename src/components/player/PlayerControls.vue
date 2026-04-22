<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const seekModel = ref(0);
const volumeModel = ref(1);

const coverArt = computed(() => playerStore.currentTrack?.cover_data_url || '');

const trackTitle = computed(() => {
	if (playerStore.currentTrack?.title) {
		return playerStore.currentTrack.title;
	}
	if (playerStore.currentPath) {
		const parts = playerStore.currentPath.split('/');
		return parts[parts.length - 1] || 'Unknown track';
	}
	return 'Arrastra una canción para reproducir';
});

const trackSubtitle = computed(() => {
	if (!playerStore.currentTrack) {
		return 'Vasak Resonance';
	}
	const artist = playerStore.currentTrack.artist || 'Unknown Artist';
	const album = playerStore.currentTrack.album || 'Unknown Album';
	return `${artist} • ${album}`;
});

const playButtonLabel = computed(() => {
	if (!playerStore.hasTrack) {
		return 'Play';
	}
	return playerStore.isPaused ? 'Resume' : 'Pause';
});

const queueLabel = computed(() => {
	const count = playerStore.queuedCount;
	if (count <= 0) {
		return '';
	}
	return count === 1 ? '1 en cola' : `${count} en cola`;
});

const queueItems = computed(() => playerStore.queue);

const draggingQueueIndex = ref<number | null>(null);

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const onQueueDragStart = (index: number) => {
	draggingQueueIndex.value = index;
};

const onQueueDragEnd = () => {
	draggingQueueIndex.value = null;
};

const onQueueDrop = (targetIndex: number) => {
	if (draggingQueueIndex.value === null) {
		return;
	}

	playerStore.moveQueueItem(draggingQueueIndex.value, targetIndex);
	draggingQueueIndex.value = null;
};

const formatSeconds = (value: number): string => {
	const safe = Math.max(0, Math.floor(value));
	const minutes = Math.floor(safe / 60)
		.toString()
		.padStart(2, '0');
	const seconds = Math.floor(safe % 60)
		.toString()
		.padStart(2, '0');
	return `${minutes}:${seconds}`;
};

const onSeekCommit = async () => {
	await playerStore.seekTo(seekModel.value);
};

const onVolumeCommit = async () => {
	await playerStore.setVolume(volumeModel.value);
};

onMounted(async () => {
	await playerStore.initProgressListener();
});

onUnmounted(() => {
	playerStore.disposeProgressListener();
});

watch(
	() => playerStore.positionSeconds,
	(value) => {
		seekModel.value = value;
	},
	{ immediate: true }
);

watch(
	() => playerStore.volume,
	(value) => {
		volumeModel.value = value;
	},
	{ immediate: true }
);
</script>

<template>
	<section class="player-shell">
		<div class="player-glow" />
		<div class="player-content">
			<div class="cover-wrap" :class="{ 'cover-wrap-empty': !coverArt }">
				<img v-if="coverArt" :src="coverArt" alt="Carátula" class="cover-image" />
				<div v-else class="cover-fallback">VR</div>
			</div>

			<header class="track-header">
				<p class="track-title">{{ trackTitle }}</p>
				<p class="track-subtitle">{{ trackSubtitle }}</p>
				<p v-if="queueLabel" class="queue-line">{{ queueLabel }}</p>
			</header>

			<div class="timeline-wrap">
				<input
					v-model.number="seekModel"
					type="range"
					:min="0"
					:max="playerStore.durationSeconds ?? 0"
					step="1"
					class="slider"
					:disabled="!playerStore.hasTrack"
					@change="onSeekCommit"
				/>
				<div class="time-row">
					<span>{{ formatSeconds(playerStore.positionSeconds) }}</span>
					<span>{{ formatSeconds(playerStore.durationSeconds ?? 0) }}</span>
				</div>
			</div>

			<div class="controls-row">
				<button
					type="button"
					class="btn-main"
					:disabled="!playerStore.hasTrack || playerStore.busy"
					@click="playerStore.togglePlayPause"
				>
					{{ playButtonLabel }}
				</button>

				<div class="volume-wrap">
					<span>Vol</span>
					<input
						v-model.number="volumeModel"
						type="range"
						min="0"
						max="2"
						step="0.01"
						class="slider"
						@change="onVolumeCommit"
					/>
				</div>
			</div>

			<section v-if="playerStore.queuedCount > 0" class="queue-panel">
				<div class="queue-panel-header">
					<p class="queue-panel-title">Próximos temas</p>
					<button type="button" class="queue-clear-btn" @click="playerStore.clearQueue">Limpiar cola</button>
				</div>

				<ul class="queue-list">
					<li
						v-for="(path, index) in queueItems"
						:key="`${path}-${index}`"
						class="queue-list-item"
						:class="{ 'queue-list-item-next': index === 0 }"
						draggable="true"
						@dragover.prevent
						@drop="onQueueDrop(index)"
						@dragstart="onQueueDragStart(index)"
						@dragend="onQueueDragEnd"
					>
						<span class="queue-index">{{ index + 1 }}.</span>
						<div class="queue-track-wrap">
							<p v-if="index === 0" class="queue-tag">Siguiente</p>
							<span class="queue-track-name">{{ extractTrackName(path) }}</span>
						</div>
						<button
							type="button"
							class="queue-remove-btn"
							@click="playerStore.removeQueueItem(index)"
						>
							Quitar
						</button>
					</li>
				</ul>
			</section>

			<p v-if="playerStore.error" class="error-line">{{ playerStore.error }}</p>
		</div>
	</section>
</template>

<style scoped>
.player-shell {
	position: relative;
	width: min(920px, 100%);
	border: 1px solid color-mix(in srgb, var(--primary) 32%, #000);
	border-radius: calc(var(--corner-radius) + 8px);
	background: linear-gradient(170deg, #151824 0%, #0f1220 100%);
	overflow: hidden;
}

.player-glow {
	position: absolute;
	inset: -80px;
	background:
		radial-gradient(circle at 14% 10%, color-mix(in srgb, var(--primary) 28%, transparent) 0 22%, transparent 45%),
		radial-gradient(circle at 85% 85%, color-mix(in srgb, var(--secondary) 20%, transparent) 0 26%, transparent 45%);
	pointer-events: none;
}

.player-content {
	position: relative;
	display: grid;
	grid-template-columns: 92px 1fr;
	align-items: start;
	gap: 1rem;
	padding: 1.1rem 1rem 1rem;
}

.cover-wrap {
	width: 92px;
	height: 92px;
	border-radius: calc(var(--corner-radius) + 2px);
	overflow: hidden;
	border: 1px solid color-mix(in srgb, var(--primary) 24%, #2f3344);
	background: #131828;
	box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.02);
}

.cover-wrap-empty {
	display: grid;
	place-items: center;
}

.cover-image {
	width: 100%;
	height: 100%;
	object-fit: cover;
}

.cover-fallback {
	font-size: 1.05rem;
	font-weight: 700;
	letter-spacing: 0.08em;
	color: color-mix(in srgb, var(--primary) 86%, #edf1ff);
}

.track-header,
.timeline-wrap,
.controls-row,
.queue-panel,
.error-line {
	grid-column: 2;
}

.track-header {
	display: grid;
	gap: 0.2rem;
}

.track-title {
	margin: 0;
	font-weight: 600;
	font-size: 0.98rem;
	color: #f2f4f8;
	letter-spacing: 0.01em;
	white-space: nowrap;
	overflow: hidden;
	text-overflow: ellipsis;
}

.track-subtitle {
	margin: 0;
	font-size: 0.8rem;
	color: #a8b0c6;
	white-space: nowrap;
	overflow: hidden;
	text-overflow: ellipsis;
}

.queue-line {
	margin: 0;
	font-size: 0.72rem;
	color: color-mix(in srgb, var(--primary) 88%, #f2f4f8);
	letter-spacing: 0.03em;
	text-transform: uppercase;
}

.timeline-wrap {
	display: grid;
	gap: 0.5rem;
}

.time-row {
	display: flex;
	justify-content: space-between;
	font-size: 0.75rem;
	color: #9aa3ba;
}

.controls-row {
	display: grid;
	grid-template-columns: auto 1fr;
	gap: 1rem;
	align-items: center;
}

.btn-main {
	padding: 0.52rem 1rem;
	border-radius: var(--corner-radius);
	border: 1px solid color-mix(in srgb, var(--primary) 52%, #000);
	background: linear-gradient(180deg, color-mix(in srgb, var(--primary) 85%, #171a27), color-mix(in srgb, var(--primary) 68%, #0f1220));
	color: #11131c;
	font-weight: 700;
	font-size: 0.82rem;
	cursor: pointer;
}

.btn-main:disabled {
	opacity: 0.45;
	cursor: not-allowed;
}

.volume-wrap {
	display: grid;
	grid-template-columns: auto 1fr;
	align-items: center;
	gap: 0.65rem;
	color: #c5cbe0;
	font-size: 0.78rem;
}

.slider {
	width: 100%;
	accent-color: var(--primary);
}

.error-line {
	margin: 0;
	font-size: 0.78rem;
	color: #ff8f95;
}

.queue-panel {
	display: grid;
	gap: 0.45rem;
	padding: 0.65rem;
	border: 1px solid color-mix(in srgb, var(--primary) 20%, #2f3344);
	border-radius: var(--corner-radius);
	background: linear-gradient(180deg, #101423, #0e1220);
}

.queue-panel-header {
	display: flex;
	justify-content: space-between;
	align-items: center;
	gap: 0.5rem;
}

.queue-panel-title {
	margin: 0;
	font-size: 0.78rem;
	font-weight: 600;
	color: #cdd3e8;
	letter-spacing: 0.04em;
	text-transform: uppercase;
}

.queue-clear-btn {
	padding: 0.32rem 0.58rem;
	border-radius: calc(var(--corner-radius) - 2px);
	border: 1px solid color-mix(in srgb, var(--secondary) 35%, #2b3044);
	background: #161a2b;
	color: #d7ddf2;
	font-size: 0.72rem;
	font-weight: 600;
	cursor: pointer;
}

.queue-list-item-next {
	background: color-mix(in srgb, var(--primary) 15%, #111726);
	border: 1px solid color-mix(in srgb, var(--primary) 30%, transparent);
}

.queue-tag {
	margin: 0;
	font-size: 0.68rem;
	font-weight: 700;
	text-transform: uppercase;
	letter-spacing: 0.05em;
	color: color-mix(in srgb, var(--primary) 88%, #edf1ff);
}

.queue-list {
	list-style: none;
	margin: 0;
	padding: 0;
	display: grid;
	gap: 0.35rem;
}

.queue-list-item {
	display: grid;
	grid-template-columns: auto 1fr auto;
	align-items: center;
	gap: 0.45rem;
	padding: 0.35rem 0.5rem;
	border-radius: calc(var(--corner-radius) - 2px);
	background: #14192a;
	border: 1px solid color-mix(in srgb, var(--secondary) 18%, #2a3043);
	cursor: grab;
}

.queue-list-item:active {
	cursor: grabbing;
}

.queue-index {
	font-size: 0.72rem;
	color: #99a2bc;
	font-weight: 600;
}

.queue-track-name {
	margin: 0;
	font-size: 0.76rem;
	color: #dde3f6;
	white-space: nowrap;
	overflow: hidden;
	text-overflow: ellipsis;
}

.queue-track-wrap {
	display: grid;
	gap: 0.1rem;
	min-width: 0;
}

.queue-remove-btn {
	padding: 0.24rem 0.45rem;
	border-radius: calc(var(--corner-radius) - 4px);
	border: 1px solid color-mix(in srgb, var(--status-error, #d20f39) 40%, #263048);
	background: #1c2133;
	color: #f2b6bf;
	font-size: 0.68rem;
	font-weight: 600;
	cursor: pointer;
}

@media (max-width: 640px) {
	.player-content {
		grid-template-columns: 1fr;
	}

	.cover-wrap {
		width: 88px;
		height: 88px;
	}

	.track-header,
	.timeline-wrap,
	.controls-row,
	.queue-panel,
	.error-line {
		grid-column: 1;
	}

	.controls-row {
		grid-template-columns: 1fr;
	}
}
</style>
