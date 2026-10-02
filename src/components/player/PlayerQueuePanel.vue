<script setup lang="ts">
import { type MenuEntry, useContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, ListRow, Panel, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { ref } from 'vue';
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
	<!-- El menú del clic derecho escucha acá y no en la lista: `TransitionGroup`
	     no declara eventos del DOM, así que el `@contextmenu` le caía encima por
	     atributos y terminaba igual en el `<ul>` que dibuja. Escuchando en la
	     sección es lo mismo —el manejador busca la fila con `closest()` y se va
	     si no hay ninguna— y además queda dicho dónde está puesto. -->
	<section class="min-w-0" @contextmenu="onQueueContextMenu">
	<Panel>
		<div class="flex min-w-0 flex-wrap items-center justify-between gap-3 pb-3">
			<div class="min-w-0">
				<p class="text-xs uppercase tracking-[0.18em] text-tx-muted">{{ t('queue.eyebrow') }}</p>
				<p class="text-sm font-medium text-tx-main">{{ t('queue.subtitle') }}</p>
			</div>
			<ActionButton
				:label="t('queue.clear')"
				icon="edit-clear-all-symbolic"
				variant="secondary"
				@click="emit('clear')"
			/>
		</div>

		<!-- Un solo menú para toda la cola; cada elemento dice cuál es el suyo
		     con `data-queue-id`. El `li` lleva el arrastre; la fila es `ListRow`. -->
		<TransitionGroup
			tag="ul"
			class="grid gap-2"
			move-class="transition-transform duration-200 ease-ui"
			enter-active-class="transition-[opacity,translate] duration-200 ease-ui-out"
			leave-active-class="transition-[opacity,translate] duration-150 ease-ui"
			enter-from-class="opacity-0 translate-y-2"
			leave-to-class="opacity-0 translate-y-2"
		>
			<li
				v-for="(entry, index) in props.queueItems"
				:key="entry.id"
				:data-queue-id="entry.id"
				class="rounded-corner-m border transition-colors duration-200 ease-ui"
				:class="[
					dropTargetId === entry.id ? 'border-primary bg-ui-selected-accent' : index === 0 ? 'border-secondary' : 'border-ui-line',
					{ 'opacity-70': draggingQueueId === entry.id },
				]"
				draggable="true"
				@dragover.prevent
				@dragenter.prevent="onQueueDragEnter(entry.id)"
				@dragleave="onQueueDragLeave(entry.id)"
				@drop="onQueueDrop(entry.id)"
				@dragstart="onQueueDragStart(entry.id)"
				@dragend="onQueueDragEnd"
			>
				<ListRow>
					<template #leading>
						<span class="w-6 shrink-0 text-right text-xs font-semibold text-tx-muted tabular-nums">{{ index + 1 }}</span>
						<span
							class="ml-3 flex size-8 shrink-0 cursor-grab items-center justify-center rounded-corner-m text-tx-muted"
							:title="t('queue.dragToReorder')"
						>
							<ThemeIcon name="list-drag-handle-symbolic" type="symbol" :size="16" :alt="t('queue.dragToReorder')" />
						</span>
					</template>
					<span v-if="index === 0" class="text-[11px] uppercase tracking-[0.2em] text-tx-muted">{{ t('queue.nextUp') }}</span>
					<span class="truncate text-label-m">{{ extractTrackName(entry.path) }}</span>
					<template #trailing>
						<ActionButton :label="t('common.remove')" variant="ghost" size="sm" @click="emit('remove', entry.id)" />
					</template>
				</ListRow>
			</li>
		</TransitionGroup>
	</Panel>
	</section>
</template>
