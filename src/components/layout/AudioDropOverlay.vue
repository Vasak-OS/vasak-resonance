<script setup lang="ts">
defineProps<{
	isActive: boolean;
	message?: string;
}>();
</script>

<template>
	<Transition
		enter-active-class="transition-opacity duration-180"
		leave-active-class="transition-opacity duration-130"
		enter-from-class="opacity-0"
		leave-to-class="opacity-0"
	>
		<div
			v-if="isActive"
			class="pointer-events-none absolute inset-3 z-20 flex items-center justify-center rounded-corner border-2 border-dashed border-primary/60 bg-primary/10"
		>
			<p class="px-4 text-center text-sm font-medium tracking-wide text-tx-main sm:text-base">
				{{ message || 'Suelta un archivo de audio para reproducir' }}
			</p>
		</div>
	</Transition>
</template>
