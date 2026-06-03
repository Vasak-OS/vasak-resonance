<script setup lang="ts">
import { ref } from 'vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';

const props = defineProps<{
	queueItems: string[];
}>();

const emit = defineEmits<{
	clear: [];
	remove: [index: number];
	reorder: [fromIndex: number, toIndex: number];
}>();

const draggingQueueIndex = ref<number | null>(null);
const dropTargetIndex = ref<number | null>(null);
const clearAllIcon = useReactiveIcon('edit-clear-all-symbolic');

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
	dropTargetIndex.value = null;
};

const onQueueDragEnter = (targetIndex: number) => {
	if (draggingQueueIndex.value === null || draggingQueueIndex.value === targetIndex) {
		dropTargetIndex.value = null;
		return;
	}

	dropTargetIndex.value = targetIndex;
};

const onQueueDragLeave = (targetIndex: number) => {
	if (dropTargetIndex.value === targetIndex) {
		dropTargetIndex.value = null;
	}
};

const onQueueDrop = (targetIndex: number) => {
	if (draggingQueueIndex.value === null) {
		return;
	}

	emit('reorder', draggingQueueIndex.value, targetIndex);
	draggingQueueIndex.value = null;
	dropTargetIndex.value = null;
};
</script>

<template>
	<section class="rounded-corner border border-ui-border bg-ui-bg/80 p-4">
		<div class="flex items-center justify-between gap-3 pb-3">
			<div>
				<p class="text-xs uppercase tracking-[0.18em] text-tx-muted">Cola</p>
				<p class="text-sm font-medium text-tx-main">Próximos temas</p>
			</div>
			<button
				type="button"
				class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
				title="Limpiar cola"
				aria-label="Limpiar cola"
				@click="emit('clear')"
			>
				<img v-if="clearAllIcon" :src="clearAllIcon" alt="Limpiar cola" class="h-4 w-4">
				Limpiar cola
			</button>
		</div>

		<TransitionGroup
			tag="ul"
			class="grid gap-2"
			move-class="transition-transform duration-200 ease-out"
			enter-active-class="transition-all duration-200 ease-out"
			leave-active-class="transition-all duration-150 ease-in"
			enter-from-class="opacity-0 translate-y-2"
			leave-to-class="opacity-0 translate-y-2"
		>
			<li
				v-for="(path, index) in props.queueItems"
				:key="`${path}-${index}`"
				class="group flex items-center gap-3 rounded-corner border border-ui-border/80 bg-ui-surface/45 px-3 py-2.5 text-sm transition-all duration-200 hover:border-primary/35 hover:bg-ui-surface/70"
				:class="{
					'border-primary/55 bg-primary/10': dropTargetIndex === index,
					'opacity-70': draggingQueueIndex === index,
					'border-secondary/40': index === 0,
				}"
				draggable="true"
				@dragover.prevent
				@dragenter.prevent="onQueueDragEnter(index)"
				@dragleave="onQueueDragLeave(index)"
				@drop="onQueueDrop(index)"
				@dragstart="onQueueDragStart(index)"
				@dragend="onQueueDragEnd"
			>
				<span class="w-6 shrink-0 text-right text-xs font-semibold text-tx-muted">{{ index + 1 }}</span>
				<span
					class="flex h-8 w-8 shrink-0 items-center justify-center rounded-corner border border-ui-border bg-ui-bg/70 text-xs font-bold tracking-[0.2em] text-primary transition-colors duration-200 group-hover:bg-primary/10 group-hover:text-tx-main"
					title="Arrastra para reordenar"
					aria-label="Arrastrar para reordenar"
				>
					⋮⋮
				</span>
				<div class="min-w-0 flex-1">
					<p v-if="index === 0" class="text-[11px] uppercase tracking-[0.2em] text-primary">Siguiente</p>
					<p class="truncate text-sm text-tx-main">{{ extractTrackName(path) }}</p>
				</div>
				<button
					type="button"
					class="rounded-corner border border-transparent bg-ui-bg/35 px-2.5 py-1 text-xs font-medium text-tx-muted transition-colors duration-200 hover:border-primary/30 hover:bg-primary/10 hover:text-tx-main"
					@click="emit('remove', index)"
				>
					Quitar
				</button>
			</li>
		</TransitionGroup>
	</section>
</template>
