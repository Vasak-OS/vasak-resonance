<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { LoadingState } from '@vasakgroup/vue-libvasak';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
</script>

<template>
	<Teleport to="body">
		<Transition name="fade">
			<div
				v-show="playerStore.isScanning"
				class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
			>
				<div class="flex flex-col items-center gap-2 rounded-corner bg-bg-primary p-8 shadow-2xl">
					<LoadingState :label="t('scanning.title')" />
					<p class="text-center text-tx-muted text-xs">{{ t('scanning.subtitle') }}</p>
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
