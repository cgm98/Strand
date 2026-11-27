<script setup lang="ts">
// Diálogo modal sobre <dialog> nativo: showModal() atrapa el foco, Esc lo cierra
// y el resto de la página queda inerte para lectores de pantalla.
// Se monta con v-if: al desmontarse, el diálogo desaparece.
import { onBeforeUnmount, onMounted, ref, useId } from "vue";
import { X } from "lucide-vue-next";

withDefaults(defineProps<{ title: string; size?: "sm" | "md" | "lg"; placement?: "center" | "top" }>(), {
  size: "md",
  placement: "center",
});
const emit = defineEmits<{ (e: "close"): void }>();
const dialog = ref<HTMLDialogElement>();
const titleId = useId();
// Devolver el foco a donde estaba al cerrar (WebKitGTK no siempre lo hace solo).
const previous = document.activeElement as HTMLElement | null;

onMounted(() => dialog.value?.showModal());
onBeforeUnmount(() => {
  dialog.value?.close();
  previous?.focus?.();
});
</script>

<template>
  <Teleport to="body">
    <dialog
      ref="dialog"
      class="modal"
      :class="[`modal-${size}`, `modal-${placement}`]"
      :aria-labelledby="titleId"
      @cancel.prevent="emit('close')"
      @click.self="emit('close')"
    >
      <div class="modal-card">
        <header class="modal-header">
          <h2 :id="titleId">{{ title }}</h2>
          <button type="button" class="icon-btn" aria-label="Cerrar" @click="emit('close')"><X :size="16" aria-hidden="true" /></button>
        </header>
        <div class="modal-body"><slot /></div>
        <footer v-if="$slots.footer" class="modal-footer"><slot name="footer" /></footer>
      </div>
    </dialog>
  </Teleport>
</template>

<style>
.modal {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--fg);
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 32px);
  overflow: visible;
}
.modal::backdrop {
  background: rgb(0 0 0 / 0.45);
}
.modal-top {
  margin-top: 12vh;
}
.modal-sm .modal-card {
  width: 420px;
}
.modal-md .modal-card {
  width: 560px;
}
.modal-lg .modal-card {
  width: 880px;
}
.modal-card {
  display: flex;
  flex-direction: column;
  max-width: 100%;
  max-height: calc(100vh - 32px);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
  overflow: hidden;
}
.modal-top .modal-card {
  max-height: 76vh;
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 12px 12px 18px;
  border-bottom: 1px solid var(--border);
}
.modal-header h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 650;
}
.modal-body {
  padding: 16px 18px;
  overflow: auto;
  min-height: 0;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
  background: var(--surface);
}
</style>
