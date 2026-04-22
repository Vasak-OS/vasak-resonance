<script setup lang="ts">
defineProps<{
	coverArt: string;
	title: string;
	subtitle: string;
	queueLabel?: string;
}>();
</script>

<template>
	<section
		class="flex min-w-0 flex-col gap-4 rounded-[calc(var(--corner-radius)+8px)] border border-ui-border bg-ui-bg/80 p-4 shadow-[0_18px_40px_rgba(0,0,0,0.25)] backdrop-blur-sm sm:flex-row sm:items-center"
	>
		<div
			class="flex h-24 w-24 shrink-0 items-center justify-center overflow-hidden rounded-[calc(var(--corner-radius)+4px)] border border-ui-border bg-ui-surface/55 ring-1 ring-white/5 sm:h-28 sm:w-28"
		>
			<img
				v-if="coverArt"
				:src="coverArt"
				:alt="title"
				class="h-full w-full object-cover"
			/>
			<div v-else class="grid h-full w-full place-items-center text-lg font-bold tracking-[0.18em] text-tx-muted">
				VR
			</div>
		</div>

		<div class="min-w-0 flex-1 space-y-1">
			<p class="truncate text-base font-semibold tracking-wide text-tx-main sm:text-lg">
				{{ title }}
			</p>
			<p class="truncate text-sm text-tx-muted">
				{{ subtitle }}
			</p>
			<p
				v-if="queueLabel"
				class="inline-flex rounded-full border border-primary/30 bg-primary/10 px-3 py-1 text-[11px] font-medium uppercase tracking-[0.18em] text-primary"
			>
				{{ queueLabel }}
			</p>
		</div>
	</section>
</template>
