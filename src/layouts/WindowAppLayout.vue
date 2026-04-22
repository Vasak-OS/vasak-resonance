<script lang="ts" setup>
import { computed } from 'vue';
import PlayerControls from '@/components/player/PlayerControls.vue';
import { audioDropDirective } from '@/directives/audioDrop';
import { usePlayerStore } from '@/stores/player';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';

const playerStore = usePlayerStore();

const dropBinding = computed(() => ({
	onFilesDropped: (paths: string[]) => playerStore.handleDroppedPaths(paths),
	onDragStateChange: (dragging: boolean) => playerStore.setDragOver(dragging),
}));

const vAudioDrop = audioDropDirective;
</script>
<template>
  <div
    class="h-screen w-screen bg-ui-bg/80 rounded-corner-window flex flex-col border border-ui-border overflow-hidden">
    <TopBarComponent>
    </TopBarComponent>
    <main v-audio-drop="dropBinding" class="relative flex-1 flex items-center justify-center p-3 bg-[radial-gradient(circle_at_top_left,#1a1e2a_0%,#0f121d_55%,#0c0f19_100%)]">
      <Transition
        enter-active-class="transition-opacity duration-180"
        leave-active-class="transition-opacity duration-130"
        enter-from-class="opacity-0"
        leave-to-class="opacity-0"
      >
        <div
          v-if="playerStore.isDragOver"
          class="absolute inset-3 z-20 rounded-[calc(var(--corner-radius)+10px)] border-2 border-dashed border-primary/60 bg-primary/10 backdrop-blur-[2px] flex items-center justify-center pointer-events-none"
        >
          <p class="text-sm font-medium tracking-wide text-tx-main">Suelta un archivo de audio para reproducir</p>
        </div>
      </Transition>

      <PlayerControls class="z-10" />
    </main>
  </div>
</template>
