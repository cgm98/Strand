<script setup lang="ts">
// Visor a pantalla completa de una imagen. Sobre <dialog> nativo: Esc cierra
// y el foco vuelve a donde estaba.
import { onBeforeUnmount, onMounted, ref } from "vue";
import { X, ZoomIn, ZoomOut, RefreshCcw } from "lucide-vue-next";
import { useZoomPan } from "../composables/useZoomPan";

defineProps<{ src: string; alt: string }>();
const emit = defineEmits<{ (e: "close"): void }>();
const dialog = ref<HTMLDialogElement>();
const zoomImg = ref<HTMLElement>();
const zoom = useZoomPan(zoomImg);
const previous = document.activeElement as HTMLElement | null;

onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => {
  dialog.value?.close();
  previous?.focus?.();
});

function onViewportClick(event: MouseEvent) {
  if (event.target === event.currentTarget) emit("close");
}
</script>

<template>
  <Teleport to="body">
    <dialog
      ref="dialog"
      class="lightbox"
      :aria-label="alt ? `Imagen ampliada: ${alt}` : 'Imagen ampliada'"
      @cancel.prevent="emit('close')"
      @keydown="zoom.onKeydown"
    >
      <div
        class="lightbox-viewport"
        @click="onViewportClick"
        @wheel="zoom.onWheel"
        @pointerdown="zoom.onPointerDown"
        @pointermove="zoom.onPointerMove"
        @pointerup="zoom.onPointerUp"
        @pointercancel="zoom.onPointerUp"
        @dblclick="zoom.onDoubleClick"
      >
        <figure class="lightbox-figure">
          <img ref="zoomImg" :src="src" :alt="alt" :style="zoom.transformStyle()" />
          <figcaption v-if="alt">{{ alt }}</figcaption>
        </figure>
      </div>
      <div class="lightbox-toolbar">
        <button type="button" aria-label="Alejar" title="Alejar (-)" @click="zoom.zoomOut">
          <ZoomOut :size="18" aria-hidden="true" />
        </button>
        <span class="lightbox-zoom-level" aria-hidden="true">{{ Math.round(zoom.state.scale * 100) }}%</span>
        <button type="button" aria-label="Acercar" title="Acercar (+)" @click="zoom.zoomIn">
          <ZoomIn :size="18" aria-hidden="true" />
        </button>
        <button type="button" aria-label="Restablecer zoom" title="Restablecer zoom (0)" @click="zoom.reset">
          <RefreshCcw :size="18" aria-hidden="true" />
        </button>
      </div>
      <button type="button" class="lightbox-close" aria-label="Cerrar" title="Cerrar (Esc)" @click="emit('close')">
        <X :size="20" aria-hidden="true" />
      </button>
    </dialog>
  </Teleport>
</template>

<style>
.lightbox {
  width: 100vw;
  height: 100vh;
  max-width: none;
  max-height: none;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  position: relative;
}
.lightbox::backdrop {
  background: rgb(0 0 0 / 0.82);
}
.lightbox-viewport {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  touch-action: none;
}
.lightbox-figure {
  margin: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  pointer-events: none;
}
.lightbox-figure img {
  max-width: calc(100vw - 64px);
  max-height: calc(100vh - 110px);
  object-fit: contain;
  border-radius: 6px;
  background: #fff;
  pointer-events: auto;
}
.lightbox-figure figcaption {
  color: #f1f1f1;
  font-size: 14px;
  text-align: center;
}
.lightbox-close {
  position: fixed;
  top: 16px;
  right: 16px;
  display: inline-flex;
  padding: 8px;
  border-radius: 999px;
  color: #fff;
  background: rgb(255 255 255 / 0.12);
}
.lightbox-close:hover {
  background: rgb(255 255 255 / 0.24);
}
.lightbox-toolbar {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px;
  border-radius: 10px;
  background: rgb(0 0 0 / 0.55);
}
.lightbox-toolbar button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 7px;
  color: #fff;
  background: rgb(255 255 255 / 0.12);
}
.lightbox-toolbar button:hover {
  background: rgb(255 255 255 / 0.24);
}
.lightbox-zoom-level {
  min-width: 3.2em;
  text-align: center;
  font-size: 12px;
  color: #f1f1f1;
}
</style>
