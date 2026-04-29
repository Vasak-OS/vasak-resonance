<script setup lang="ts">
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
</script>

<template>
	<Teleport to="body">
		<Transition name="fade">
			<div
				v-show="playerStore.isScanning"
				class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
			>
				<div class="flex flex-col items-center gap-4 rounded-lg bg-bg-primary p-8 shadow-2xl">
					<!-- Spinner SVG -->
					<svg
						class="h-12 w-12 animate-spin text-primary"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
					>
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
						<path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						/>
					</svg>

					<!-- Text -->
					<div class="text-center">
						<p class="text-sm font-medium text-tx-primary">Indexando biblioteca musical...</p>
						<p class="mt-1 text-xs text-tx-muted">Por favor espera...</p>
					</div>
				</div>
			</div>
		</Transition>
	</Teleport>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
	transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
	opacity: 0;
}
</style>
