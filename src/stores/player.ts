import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import {
	handleDroppedFile,
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
	const positionSeconds = ref(0);
	const durationSeconds = ref<number | null>(null);
	const isPlaying = ref(false);
	const isPaused = ref(false);
	const volume = ref(1);
	const busy = ref(false);
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

	const applyProgress = (payload: PlaybackProgressEvent) => {
		currentPath.value = payload.path;
		positionSeconds.value = payload.position_seconds;
		durationSeconds.value = payload.duration_seconds;
		isPlaying.value = payload.is_playing;
		isPaused.value = payload.is_paused;
		volume.value = payload.volume;
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

	const handleDroppedPaths = async (paths: string[]) => {
		const [firstPath] = paths;
		if (!firstPath) {
			return;
		}
		await playDropped(firstPath);
	};

	return {
		busy,
		currentPath,
		currentTrack,
		durationSeconds,
		error,
		handleDroppedPaths,
		hasTrack,
		initProgressListener,
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
