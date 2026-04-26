<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
	defineProps<{
		title: string;
		subtitle: string;
		coverSrc?: string | null;
		variant?: 'compact' | 'stacked';
		placeholderText?: string;
		titleClass?: string;
		subtitleClass?: string;
	}>(),
	{
		coverSrc: null,
		variant: 'compact',
		placeholderText: 'Sin portada',
		titleClass: 'text-tx-main',
		subtitleClass: 'text-tx-muted',
	}
);

const rootClass = computed(() => {
	return props.variant === 'stacked'
		? 'flex flex-col items-center'
		: 'flex min-h-0 flex-1 items-center gap-3';
});

const coverWrapperClass = computed(() => {
	return props.variant === 'stacked'
		? 'mx-auto mb-3 flex h-36 w-full max-w-55 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-bg/60'
		: 'h-16 w-16 shrink-0 overflow-hidden rounded-corner border border-primary/25 bg-ui-bg/70';
});

const metaClass = computed(() => {
	return props.variant === 'stacked' ? 'mb-3 w-full space-y-1' : 'min-w-0 flex-1';
});
</script>

<template>
	<div :class="rootClass">
		<div :class="coverWrapperClass">
			<img v-if="coverSrc" :src="coverSrc" :alt="title" class="h-full w-full object-cover">
			<div v-else class="flex h-full w-full items-center justify-center text-[10px] font-semibold uppercase tracking-[0.14em] text-tx-muted">
				{{ placeholderText }}
			</div>
		</div>

		<div :class="metaClass">
			<p class="truncate text-sm font-semibold" :class="titleClass">{{ title }}</p>
			<p class="truncate text-xs" :class="subtitleClass">{{ subtitle }}</p>
		</div>
	</div>
</template>
