import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import {
	handleDroppedFile,
	type NowPlayingMetadata,
	pausePlayback,
	playFile,
	resumePlayback,
	seekPlayback,
	setPlaybackVolume,
	type DroppedPlaybackTrack,
	type PlaybackProgressEvent,
} from '@/services/player.service';

export const usePlayerStore = defineStore('player', () => {
	const currentTrack = ref<DroppedPlaybackTrack | null>(null);
	const currentPath = ref<string | null>(null);
	const queue = ref<string[]>([]);
	const positionSeconds = ref(0);
	const durationSeconds = ref<number | null>(null);
	const isPlaying = ref(false);
	const isPaused = ref(false);
	const volume = ref(1);
	const busy = ref(false);
	const isAdvancingQueue = ref(false);
	const lastAutoAdvancedPath = ref<string | null>(null);
	const error = ref('');
	const isDragOver = ref(false);
	let unlistenProgress: UnlistenFn | null = null;

	const progressPercent = computed(() => {
		if (!durationSeconds.value || durationSeconds.value <= 0) {
			return 0;
		}
		return Math.min(100, (positionSeconds.value / durationSeconds.value) * 100);
	});

	const hasTrack = computed(() => Boolean(currentPath.value));
	const queuedCount = computed(() => queue.value.length);

	const shouldAutoAdvanceQueue = (payload: PlaybackProgressEvent): boolean => {
		if (queue.value.length === 0 || isAdvancingQueue.value) {
			return false;
		}

		if (!payload.path || payload.is_playing || payload.is_paused) {
			return false;
		}

		if (payload.duration_seconds === null) {
			return false;
		}

		if (payload.position_seconds < payload.duration_seconds) {
			return false;
		}

		return lastAutoAdvancedPath.value !== payload.path;
	};

	const playNextInQueue = async () => {
		const [nextPath, ...rest] = queue.value;
		if (!nextPath) {
			return;
		}

		queue.value = rest;
		isAdvancingQueue.value = true;
		try {
			await playDropped(nextPath);
		} catch {
			if (queue.value.length > 0) {
				await playNextInQueue();
			}
		} finally {
			isAdvancingQueue.value = false;
		}
	};

	const applyProgress = (payload: PlaybackProgressEvent) => {
		currentPath.value = payload.path;
		positionSeconds.value = payload.position_seconds;
		durationSeconds.value = payload.duration_seconds;
		isPlaying.value = payload.is_playing;
		isPaused.value = payload.is_paused;
		volume.value = payload.volume;

		if (payload.now_playing) {
			const nowPlaying: NowPlayingMetadata = payload.now_playing;
			currentTrack.value = {
				path: nowPlaying.path,
				title: nowPlaying.title,
				artist: nowPlaying.artist,
				album: nowPlaying.album,
				duration_seconds: nowPlaying.duration_seconds,
				cover_data_url: nowPlaying.cover_data_url,
			};
		}

		if (payload.is_playing) {
			lastAutoAdvancedPath.value = null;
			return;
		}

		if (shouldAutoAdvanceQueue(payload)) {
			lastAutoAdvancedPath.value = payload.path;
			void playNextInQueue();
		}
	};

	const initProgressListener = async () => {
		if (unlistenProgress) {
			return;
		}

		unlistenProgress = await listen<PlaybackProgressEvent>('audio-playback-progress', (event) => {
			applyProgress(event.payload);
		});
	};

	const disposeProgressListener = () => {
		if (unlistenProgress) {
			unlistenProgress();
			unlistenProgress = null;
		}
	};

	const play = async (filePath: string) => {
		busy.value = true;
		error.value = '';
		try {
			await playFile(filePath);
			currentPath.value = filePath;
		} catch (playError: unknown) {
			error.value = `No se pudo reproducir el archivo: ${String(playError)}`;
		} finally {
			busy.value = false;
		}
	};

	const playDropped = async (filePath: string) => {
		busy.value = true;
		error.value = '';
		try {
			const track = await handleDroppedFile(filePath);
			currentTrack.value = track;
			currentPath.value = track.path;
			durationSeconds.value = track.duration_seconds;
			await playFile(track.path);
		} catch (dropError: unknown) {
			error.value = `No se pudo cargar el archivo arrastrado: ${String(dropError)}`;
		} finally {
			busy.value = false;
		}
	};

	const pause = async () => {
		try {
			error.value = '';
			await pausePlayback();
		} catch (pauseError: unknown) {
			error.value = `No se pudo pausar: ${String(pauseError)}`;
		}
	};

	const resume = async () => {
		try {
			error.value = '';
			await resumePlayback();
		} catch (resumeError: unknown) {
			error.value = `No se pudo reanudar: ${String(resumeError)}`;
		}
	};

	const togglePlayPause = async () => {
		if (!hasTrack.value) {
			return;
		}
		if (isPaused.value) {
			await resume();
		} else {
			await pause();
		}
	};

	const seekTo = async (seconds: number) => {
		try {
			error.value = '';
			await seekPlayback(seconds);
		} catch (seekError: unknown) {
			error.value = `No se pudo mover la reproducción: ${String(seekError)}`;
		}
	};

	const setVolume = async (nextVolume: number) => {
		const normalized = Math.min(2, Math.max(0, nextVolume));
		volume.value = normalized;
		try {
			error.value = '';
			await setPlaybackVolume(normalized);
		} catch (volumeError: unknown) {
			error.value = `No se pudo ajustar el volumen: ${String(volumeError)}`;
		}
	};

	const setDragOver = (value: boolean) => {
		isDragOver.value = value;
	};

	const clearQueue = () => {
		queue.value = [];
	};

	const removeQueueItem = (index: number) => {
		if (index < 0 || index >= queue.value.length) {
			return;
		}

		queue.value = queue.value.filter((_, itemIndex) => itemIndex !== index);
	};

	const moveQueueItem = (fromIndex: number, toIndex: number) => {
		if (
			fromIndex < 0 ||
			fromIndex >= queue.value.length ||
			toIndex < 0 ||
			toIndex >= queue.value.length ||
			fromIndex === toIndex
		) {
			return;
		}

		const nextQueue = [...queue.value];
		const [movedItem] = nextQueue.splice(fromIndex, 1);
		nextQueue.splice(toIndex, 0, movedItem);
		queue.value = nextQueue;
	};

	const handleDroppedPaths = async (paths: string[]) => {
		const normalized = Array.from(new Set(paths.map((path) => path.trim()).filter((path) => path.length > 0)));
		const [firstPath, ...rest] = normalized;
		if (!firstPath) {
			return;
		}

		queue.value = rest;
		await playDropped(firstPath);
	};

	return {
		busy,
		clearQueue,
		currentPath,
		currentTrack,
		durationSeconds,
		error,
		handleDroppedPaths,
		hasTrack,
		initProgressListener,
		queue,
		queuedCount,
		moveQueueItem,
		removeQueueItem,
		isDragOver,
		isPaused,
		isPlaying,
		pause,
		play,
		playDropped,
		positionSeconds,
		progressPercent,
		resume,
		seekTo,
		setDragOver,
		setVolume,
		togglePlayPause,
		disposeProgressListener,
		volume,
	};
});
