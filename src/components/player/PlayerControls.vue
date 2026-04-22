<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const seekModel = ref(0);
const volumeModel = ref(1);

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
	gap: 1rem;
	padding: 1.1rem 1rem 1rem;
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

@media (max-width: 640px) {
	.controls-row {
		grid-template-columns: 1fr;
	}
}
</style>
