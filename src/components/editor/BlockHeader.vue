<script setup lang="ts">
// Barra superior de un bloque de código renderizado (estilo chat de IA: etiqueta a
// la izquierda, acciones a la derecha). La monta `liveDecorations.ts` dentro del
// widget de vista previa; «Eliminar» borra el bloque completo del documento.
// También la usa MermaidRenderer.vue (slot «extra» para «Ampliar»): antes tenía
// su propia barra con el mismo marcado y el mismo CSS copiado a mano.
import { ref } from "vue";
import { Check, Copy, Pencil, Trash2 } from "lucide-vue-next";

const props = defineProps<{ label?: string; copyText: string; copyTitle?: string; editTitle?: string; removeTitle?: string }>();
const emit = defineEmits<{ (e: "edit"): void; (e: "remove"): void }>();
const copied = ref(false);

async function copy() {
  await navigator.clipboard.writeText(props.copyText);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}
</script>

<template>
  <div class="cm-block-header" role="toolbar" aria-label="Acciones del bloque">
    <span v-if="label" class="cm-block-label">{{ label }}</span>
    <span class="cm-block-spacer"></span>
    <slot name="extra" />
    <button type="button" :aria-label="copied ? 'Copiado' : (copyTitle ?? 'Copiar')" :title="copied ? 'Copiado' : (copyTitle ?? 'Copiar')" @click="copy">
      <Check v-if="copied" :size="14" aria-hidden="true" />
      <Copy v-else :size="14" aria-hidden="true" />
    </button>
    <button type="button" :aria-label="editTitle ?? 'Editar'" :title="editTitle ?? 'Editar'" @click="emit('edit')">
      <Pencil :size="14" aria-hidden="true" />
    </button>
    <button type="button" class="danger" :aria-label="removeTitle ?? 'Eliminar bloque'" :title="removeTitle ?? 'Eliminar bloque'" @click="emit('remove')">
      <Trash2 :size="14" aria-hidden="true" />
    </button>
  </div>
</template>

<style scoped>
.cm-block-header {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 6px;
}
.cm-block-label {
  font-size: 11px;
  color: var(--muted);
  font-family: var(--font-mono);
}
.cm-block-spacer {
  flex: 1;
}
/* `:slotted()` para que el botón «extra» que pasa MermaidRenderer.vue (p. ej.
   «Ampliar») se vea idéntico a los que renderiza este propio componente: el
   contenido de un slot lleva el atributo de scope del padre, no el de aquí,
   así que un selector sin `:slotted()` no lo alcanzaría. */
.cm-block-header button,
.cm-block-header :slotted(button) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 5px;
  color: var(--muted);
}
.cm-block-header button:hover,
.cm-block-header :slotted(button:hover) {
  color: var(--fg);
  background: var(--surface-2);
}
.cm-block-header button.danger:hover,
.cm-block-header :slotted(button.danger:hover) {
  color: var(--danger);
}
</style>
