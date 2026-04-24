<script setup lang="ts">
const props = defineProps<{
	hasTrack: boolean;
	busy: boolean;
	isPaused: boolean;
	playLabel: string;
	positionSeconds: number;
	durationSeconds: number | null;
	seekValue: number;
	volumeValue: number;
}>();

const emit = defineEmits<{
	'toggle-play-pause': [];
	'update:seekValue': [value: number];
	'update:volumeValue': [value: number];
	'seek-commit': [];
	'volume-commit': [];
}>();

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

const onSeekInput = (event: Event) => {
	emit('update:seekValue', Number((event.target as HTMLInputElement).value));
};

const onVolumeInput = (event: Event) => {
	emit('update:volumeValue', Number((event.target as HTMLInputElement).value));
};
</script>

<template>
	<section class="grid gap-4 rounded-corner border border-ui-border bg-ui-bg/80 p-4">
		<div class="grid gap-3">
			<div class="flex items-center justify-between gap-3">
				<div class="min-w-0">
					<p class="text-xs uppercase tracking-[0.18em] text-tx-muted">Reproducción</p>
					<p class="truncate text-sm font-medium text-tx-main">
						{{ formatSeconds(positionSeconds) }} / {{ formatSeconds(durationSeconds ?? 0) }}
					</p>
				</div>
				<span
					class="rounded-full border border-ui-border bg-ui-surface/55 px-3 py-1 text-[11px] font-medium uppercase tracking-[0.16em] text-tx-muted"
				>
					{{ isPaused ? 'Pausado' : hasTrack ? 'En curso' : 'Listo' }}
				</span>
			</div>

			<input
				:type="'range'"
				:min="0"
				:max="durationSeconds ?? 0"
				step="1"
				:value="seekValue"
				:disabled="!hasTrack"
				class="h-2 w-full cursor-pointer appearance-none rounded-full accent-primary disabled:cursor-not-allowed"
				@input="onSeekInput"
				@change="emit('seek-commit')"
			/>
		</div>

		<div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
			<button
				type="button"
				class="inline-flex items-center justify-center rounded-corner border border-primary/45 bg-primary px-4 py-2 text-sm font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
				:disabled="!hasTrack || busy"
				@click="emit('toggle-play-pause')"
			>
				{{ playLabel }}
			</button>

			<div class="grid min-w-0 flex-1 gap-2">
				<div class="flex items-center justify-between gap-3 text-xs uppercase tracking-[0.18em] text-tx-muted">
					<span>Volumen</span>
					<span>{{ Math.round(volumeValue * 100) / 100 }}</span>
				</div>
				<input
					type="range"
					min="0"
					max="2"
					step="0.01"
					:value="volumeValue"
					class="h-2 w-full cursor-pointer appearance-none rounded-full accent-primary"
					@input="onVolumeInput"
					@change="emit('volume-commit')"
				/>
			</div>
		</div>
	</section>
</template>
