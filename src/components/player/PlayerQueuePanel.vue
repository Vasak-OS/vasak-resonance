<script setup lang="ts">
import { type MenuEntry, useContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import type { QueueEntry } from '@/stores/playerQueue';

const props = defineProps<{
	queueItems: QueueEntry[];
}>();

const emit = defineEmits<{
	clear: [];
	play: [id: string];
	remove: [id: string];
	reorder: [fromId: string, toId: string];
}>();

const { t } = useI18n();
const { show } = useContextMenu();

/**
 * El clic derecho sobre la cola. Se guarda el identificador de la entrada y no
 * su posición: el menú se queda abierto mientras la persona lee las opciones y
 * en ese rato la canción en curso puede terminar, con lo que la cola avanza y
 * todo corre un lugar. Con la posición, «quitar» se llevaba la canción de al
 * lado. La ruta tampoco alcanza, porque la misma canción puede estar dos veces
 * en la cola.
 */
async function onQueueContextMenu(event: MouseEvent) {
	const target = event.target;
	const row = target instanceof Element ? target.closest<HTMLElement>('[data-queue-id]') : null;
	const id = row?.dataset.queueId;

	if (id === undefined) {
		return;
	}

	event.stopPropagation();

	const entries: MenuEntry[] = [
		{ id: 'play', label: t('contextMenu.playNow'), icon: 'media-playback-start' },
		{ id: 'remove', label: t('contextMenu.removeFromQueue'), icon: 'list-remove' },
		{ type: 'separator' },
		{
			id: 'clear',
			label: t('contextMenu.clearQueue'),
			icon: 'edit-clear-all',
			danger: true,
		},
	];

	const chosen = await show(entries, event);

	switch (chosen?.id) {
		case 'play':
			emit('play', id);
			break;
		case 'remove':
			emit('remove', id);
			break;
		case 'clear':
			emit('clear');
			break;
	}
}

// Arrastrar tiene el mismo problema que el menú —la cola puede avanzar entre
// que se agarra una fila y se suelta—, así que también se recuerda por
// identificador.
const draggingQueueId = ref<string | null>(null);
const dropTargetId = ref<string | null>(null);
const clearAllIcon = useReactiveIcon('edit-clear-all-symbolic');

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const onQueueDragStart = (id: string) => {
	draggingQueueId.value = id;
};

const onQueueDragEnd = () => {
	draggingQueueId.value = null;
	dropTargetId.value = null;
};

const onQueueDragEnter = (targetId: string) => {
	if (draggingQueueId.value === null || draggingQueueId.value === targetId) {
		dropTargetId.value = null;
		return;
	}

	dropTargetId.value = targetId;
};

const onQueueDragLeave = (targetId: string) => {
	if (dropTargetId.value === targetId) {
		dropTargetId.value = null;
	}
};

const onQueueDrop = (targetId: string) => {
	if (draggingQueueId.value === null) {
		return;
	}

	emit('reorder', draggingQueueId.value, targetId);
	draggingQueueId.value = null;
	dropTargetId.value = null;
};
</script>

<template>
	<section class="rounded-corner border border-ui-border bg-ui-bg/80 p-4">
		<div class="flex items-center justify-between gap-3 pb-3">
			<div>
				<p class="text-xs uppercase tracking-[0.18em] text-tx-muted">{{ t('queue.eyebrow') }}</p>
				<p class="text-sm font-medium text-tx-main">{{ t('queue.subtitle') }}</p>
			</div>
			<button
				type="button"
				class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
				:title="t('queue.clear')"
				:aria-label="t('queue.clear')"
				@click="emit('clear')"
			>
				<img v-if="clearAllIcon" :src="clearAllIcon" :alt="t('queue.clear')" class="h-4 w-4">
				{{ t('queue.clear') }}
			</button>
		</div>

		<!-- Un solo menú para toda la cola; cada elemento dice cuál es el suyo
		     con `data-queue-id`. -->
		<TransitionGroup
			tag="ul"
			class="grid gap-2"
			@contextmenu="onQueueContextMenu"
			move-class="transition-transform duration-200 ease-out"
			enter-active-class="transition-all duration-200 ease-out"
			leave-active-class="transition-all duration-150 ease-in"
			enter-from-class="opacity-0 translate-y-2"
			leave-to-class="opacity-0 translate-y-2"
		>
			<li
				v-for="(entry, index) in props.queueItems"
				:key="entry.id"
				:data-queue-id="entry.id"
				class="group flex items-center gap-3 rounded-corner border border-ui-border/80 bg-ui-surface/45 px-3 py-2.5 text-sm transition-all duration-200 hover:border-primary/35 hover:bg-ui-surface/70"
				:class="{
					'border-primary/55 bg-primary/10': dropTargetId === entry.id,
					'opacity-70': draggingQueueId === entry.id,
					'border-secondary/40': index === 0,
				}"
				draggable="true"
				@dragover.prevent
				@dragenter.prevent="onQueueDragEnter(entry.id)"
				@dragleave="onQueueDragLeave(entry.id)"
				@drop="onQueueDrop(entry.id)"
				@dragstart="onQueueDragStart(entry.id)"
				@dragend="onQueueDragEnd"
			>
				<span class="w-6 shrink-0 text-right text-xs font-semibold text-tx-muted">{{ index + 1 }}</span>
				<span
					class="flex h-8 w-8 shrink-0 items-center justify-center rounded-corner border border-ui-border bg-ui-bg/70 text-xs font-bold tracking-[0.2em] text-primary transition-colors duration-200 group-hover:bg-primary/10 group-hover:text-tx-main"
					:title="t('queue.dragToReorder')"
					:aria-label="t('queue.dragToReorder')"
				>
					⋮⋮
				</span>
				<div class="min-w-0 flex-1">
					<p v-if="index === 0" class="text-[11px] uppercase tracking-[0.2em] text-primary">{{ t('queue.nextUp') }}</p>
					<p class="truncate text-sm text-tx-main">{{ extractTrackName(entry.path) }}</p>
				</div>
				<button
					type="button"
					class="rounded-corner border border-transparent bg-ui-bg/35 px-2.5 py-1 text-xs font-medium text-tx-muted transition-colors duration-200 hover:border-primary/30 hover:bg-primary/10 hover:text-tx-main"
					@click="emit('remove', entry.id)"
				>
					{{ t('common.remove') }}
				</button>
			</li>
		</TransitionGroup>
	</section>
</template>
