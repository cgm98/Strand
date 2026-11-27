<script setup lang="ts">
// Modo presentación: pantalla completa dentro de la propia ventana (no un
// `window.open`, que no funciona en el webview de Tauri). Cada diapositiva es
// un render estático (como la vista previa de una celda de tabla); los
// diagramas Mermaid en vivo quedan fuera de alcance de esta primera versión,
// igual que en la exportación a PDF.
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { ChevronLeft, ChevronRight, X } from "lucide-vue-next";
import { renderMarkdown } from "../lib/markdown";
import { sanitizeHtml } from "../lib/sanitize";
import { splitSlides } from "../lib/slides";

const props = defineProps<{ title: string; source: string }>();
const emit = defineEmits<{ (e: "close"): void }>();

const slides = computed(() => splitSlides(props.source));
const index = ref(0);
const html = computed(() => sanitizeHtml(renderMarkdown(slides.value[index.value] ?? "")));
const previous = document.activeElement as HTMLElement | null;

function go(delta: number) {
  index.value = Math.min(Math.max(index.value + delta, 0), slides.value.length - 1);
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  else if (e.key === "ArrowRight" || e.key === " " || e.key === "PageDown") go(1);
  else if (e.key === "ArrowLeft" || e.key === "PageUp") go(-1);
  else if (e.key === "Home") index.value = 0;
  else if (e.key === "End") index.value = slides.value.length - 1;
  else return;
  e.preventDefault();
}

onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  previous?.focus?.();
});
</script>

<template>
  <Teleport to="body">
    <div class="presentation" role="dialog" aria-modal="true" :aria-label="`Presentación: ${title}`">
      <header class="presentation-bar">
        <span class="presentation-title">{{ title }}</span>
        <span class="presentation-count">{{ index + 1 }} / {{ slides.length }}</span>
        <button type="button" class="icon-btn" aria-label="Cerrar presentación (Esc)" @click="emit('close')">
          <X :size="18" aria-hidden="true" />
        </button>
      </header>
      <div class="presentation-slide markdown-reading" v-html="html"></div>
      <nav class="presentation-nav" aria-label="Diapositivas">
        <button type="button" class="icon-btn" :disabled="index === 0" aria-label="Diapositiva anterior" @click="go(-1)">
          <ChevronLeft :size="22" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" :disabled="index === slides.length - 1" aria-label="Diapositiva siguiente" @click="go(1)">
          <ChevronRight :size="22" aria-hidden="true" />
        </button>
      </nav>
    </div>
  </Teleport>
</template>

<style scoped>
.presentation {
  position: fixed;
  inset: 0;
  z-index: 9500;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  color: var(--fg);
}
.presentation-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.presentation-title {
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.presentation-count {
  margin-left: auto;
  color: var(--muted);
  font-size: 12px;
}
.presentation-slide {
  flex: 1;
  overflow: auto;
  padding: 48px 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.presentation-slide :deep(> *) {
  max-width: 760px;
  width: 100%;
  font-size: 1.15em;
}
.presentation-nav {
  display: flex;
  justify-content: space-between;
  padding: 10px 18px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}
</style>
