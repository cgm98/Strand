<script setup lang="ts">
// Portado de cogni-vault. Cambio: «Descargar SVG» pasa a «Copiar SVG», porque
// en Tauri un <a download> no hace nada y no tenemos plugin de sistema de archivos.
import { onBeforeUnmount, ref, watch } from "vue";
import { Maximize2, RefreshCcw, ZoomIn, ZoomOut } from "lucide-vue-next";
import { renderMermaid } from "../lib/mermaid";
import { useZoomPan } from "../composables/useZoomPan";
import BlockHeader from "./editor/BlockHeader.vue";

const props = defineProps<{ code?: string }>();
const emit = defineEmits<{ (e: "edit"): void; (e: "remove"): void }>();
const svg = ref("");
const error = ref("");
const loading = ref(false);
const dialog = ref<HTMLDialogElement>();
const zoomCanvas = ref<HTMLElement>();
const zoom = useZoomPan(zoomCanvas);
let revision = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let focusBeforeDialog: HTMLElement | null = null;

function openDialog() {
  focusBeforeDialog = document.activeElement as HTMLElement | null;
  zoom.reset();
  dialog.value?.showModal();
}
function restoreFocus() {
  focusBeforeDialog?.focus?.();
  focusBeforeDialog = null;
}

watch(
  () => props.code,
  (code) => {
    const request = ++revision;
    clearTimeout(timer);
    svg.value = "";
    error.value = "";
    loading.value = !!code?.trim();
    if (!code?.trim()) return;
    timer = setTimeout(async () => {
      try {
        const result = await renderMermaid(code);
        if (request === revision) svg.value = result;
      } catch (reason) {
        if (request === revision) error.value = reason instanceof Error ? reason.message : "No se pudo dibujar el diagrama";
      } finally {
        if (request === revision) loading.value = false;
      }
    }, 150);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  ++revision;
  clearTimeout(timer);
  dialog.value?.close();
});
</script>

<template>
  <figure class="mermaid-figure" :aria-busy="loading" @click.stop>
    <div v-if="loading" class="mermaid-message" role="status">Dibujando diagrama…</div>
    <div v-else-if="error" class="mermaid-error" role="alert">
      <strong>No se pudo dibujar este diagrama</strong>
      <pre>{{ error }}</pre>
      <details>
        <summary>Ver código Mermaid</summary>
        <pre>{{ code }}</pre>
      </details>
    </div>
    <template v-else-if="svg">
      <!-- Misma cabecera que código/fórmula (BlockHeader.vue): antes era una barra
           propia que repetía su marcado y su CSS a mano. -->
      <BlockHeader
        label="mermaid"
        :copy-text="svg"
        copy-title="Copiar SVG"
        edit-title="Editar diagrama"
        remove-title="Eliminar diagrama"
        @edit="emit('edit')"
        @remove="emit('remove')"
      >
        <template #extra>
          <button type="button" aria-label="Ampliar" title="Ampliar" @click="openDialog">
            <Maximize2 :size="14" aria-hidden="true" />
          </button>
        </template>
      </BlockHeader>
      <!-- tabindex: con --block-max-height este lienzo puede quedar con scroll propio
           (document.css); sin esto no hay forma de desplazarlo solo con teclado. -->
      <div class="mermaid-canvas" role="img" tabindex="0" aria-label="Diagrama Mermaid" v-html="svg"></div>
    </template>
    <p v-else class="mermaid-message">El bloque Mermaid está vacío.</p>
    <Teleport to="body">
      <dialog
        ref="dialog"
        class="mermaid-dialog"
        aria-label="Diagrama ampliado"
        @keydown="zoom.onKeydown"
        @close="restoreFocus"
        @click="$event.target === dialog && dialog?.close()"
      >
        <div class="mermaid-dialog-toolbar">
          <button type="button" aria-label="Alejar" title="Alejar (-)" @click="zoom.zoomOut">
            <ZoomOut :size="16" aria-hidden="true" />
          </button>
          <span class="mermaid-zoom-level" aria-hidden="true">{{ Math.round(zoom.state.scale * 100) }}%</span>
          <button type="button" aria-label="Acercar" title="Acercar (+)" @click="zoom.zoomIn">
            <ZoomIn :size="16" aria-hidden="true" />
          </button>
          <button type="button" aria-label="Restablecer zoom" title="Restablecer zoom (0)" @click="zoom.reset">
            <RefreshCcw :size="16" aria-hidden="true" />
          </button>
          <button type="button" class="mermaid-close" @click="dialog?.close()">Cerrar</button>
        </div>
        <div
          class="mermaid-viewport"
          @wheel="zoom.onWheel"
          @pointerdown="zoom.onPointerDown"
          @pointermove="zoom.onPointerMove"
          @pointerup="zoom.onPointerUp"
          @pointercancel="zoom.onPointerUp"
          @dblclick="zoom.onDoubleClick"
        >
          <div ref="zoomCanvas" class="mermaid-canvas" :style="zoom.transformStyle()" role="img" aria-label="Diagrama Mermaid ampliado" v-html="svg"></div>
        </div>
      </dialog>
    </Teleport>
  </figure>
</template>

<style scoped>
.mermaid-figure {
  margin: 16px 0;
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  min-width: 0;
}
.mermaid-canvas {
  background: #fff;
  color: #172033;
  padding: 20px;
  /* Mismo techo que código y fórmulas (document.css): un diagrama muy alto se
     recorta con scroll propio en vez de estirar la nota; "Ampliar" (el diálogo
     de abajo) sigue siendo la vía para verlo entero con zoom/pan. */
  max-height: var(--block-max-height);
  overflow: auto;
  text-align: center;
}
.mermaid-canvas :deep(svg) {
  display: block;
  max-width: 100%;
  height: auto;
  margin: auto;
}
/* BlockHeader.vue ya trae su propio layout/botones (ver su <style>); aquí solo
   lo que necesita por ser el primer elemento de la figura: fondo propio (el
   lienzo de abajo es blanco a propósito, ver `.mermaid-canvas`) y el filete que
   lo separa del diagrama. `:deep()` porque es un componente hijo, no marcado
   propio de esta plantilla. */
.mermaid-figure :deep(.cm-block-header) {
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.mermaid-close {
  font-size: 12px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
}
.mermaid-message,
.mermaid-error {
  padding: 16px;
  color: var(--muted);
}
.mermaid-error strong {
  color: var(--danger);
}
.mermaid-error pre {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-size: 12px;
}
.mermaid-dialog {
  width: 90vw;
  max-width: 1400px;
  height: 90vh;
  padding: 0;
  background: var(--surface);
  color: var(--fg);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}
/* El navegador oculta un <dialog> sin `open` con `display: none`; declarar
   `display: flex` sin condición en el selector de arriba pisaba esa regla y
   dejaba el diálogo (con su diagrama duplicado) siempre visible en el flujo
   del documento, empujando el contenido de abajo. */
.mermaid-dialog[open] {
  display: flex;
  flex-direction: column;
}
.mermaid-dialog::backdrop {
  background: #000a;
}
.mermaid-dialog-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  flex: none;
}
.mermaid-dialog-toolbar button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--fg);
}
.mermaid-dialog-toolbar button:hover {
  border-color: var(--primary);
}
.mermaid-zoom-level {
  min-width: 3.5em;
  text-align: center;
  font-size: 12px;
  color: var(--muted);
}
.mermaid-close {
  width: auto;
  padding: 0 10px;
  margin-left: auto;
}
.mermaid-viewport {
  flex: 1;
  overflow: hidden;
  touch-action: none;
}
.mermaid-viewport .mermaid-canvas {
  width: 100%;
  height: 100%;
  max-height: none;
  padding: 20px;
}
.mermaid-viewport .mermaid-canvas :deep(svg) {
  max-width: none;
  width: auto;
  height: 100%;
}
</style>
