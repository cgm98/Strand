<script setup lang="ts">
// Portado de cogni-vault y ampliado: además de abrir notas por título, ejecuta
// acciones. Navegable con flechas (patrón ARIA combobox + listbox).
import { computed, nextTick, onMounted, ref } from "vue";
import { FileText, Zap } from "lucide-vue-next";
import Modal from "./Modal.vue";
import type { FileItem } from "../types";

export interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
}

const props = defineProps<{ files: FileItem[]; actions: PaletteAction[] }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "selectFile", id: string): void;
  (e: "action", id: string): void;
  (e: "createFile", title: string): void;
}>();

const query = ref("");
const activeIndex = ref(0);
const input = ref<HTMLInputElement>();

type Item = { kind: "file" | "action" | "create"; id: string; label: string; hint?: string };
const items = computed<Item[]>(() => {
  const q = query.value.trim().toLocaleLowerCase();
  const files = props.files
    .filter((f) => !q || f.name.toLocaleLowerCase().includes(q))
    .sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0))
    .slice(0, q ? 12 : 8)
    .map((f): Item => ({ kind: "file", id: f.id, label: f.name }));
  const actions = props.actions
    .filter((a) => !q || a.label.toLocaleLowerCase().includes(q))
    .map((a): Item => ({ kind: "action", id: a.id, label: a.label, hint: a.hint }));
  const exact = props.files.some((f) => f.name.toLocaleLowerCase() === q);
  const create: Item[] = q && !exact ? [{ kind: "create", id: "create", label: `Crear nota «${query.value.trim()}»` }] : [];
  return [...files, ...create, ...actions];
});

function choose(item: Item | undefined) {
  if (!item) return;
  if (item.kind === "file") emit("selectFile", item.id);
  else if (item.kind === "action") emit("action", item.id);
  else emit("createFile", query.value.trim());
  emit("close");
}

function onKey(e: KeyboardEvent) {
  const n = items.value.length;
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    if (n) activeIndex.value = (activeIndex.value + (e.key === "ArrowDown" ? 1 : -1) + n) % n;
    void nextTick(() => document.getElementById(`palette-${activeIndex.value}`)?.scrollIntoView({ block: "nearest" }));
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose(items.value[activeIndex.value]);
  }
}

onMounted(() => input.value?.focus());
</script>

<template>
  <Modal title="Abrir nota o ejecutar acción" placement="top" @close="emit('close')">
    <input
      ref="input"
      v-model="query"
      class="field palette-input"
      role="combobox"
      aria-label="Buscar nota o acción"
      aria-controls="palette-list"
      aria-expanded="true"
      aria-autocomplete="list"
      :aria-activedescendant="items.length ? `palette-${activeIndex}` : undefined"
      placeholder="Título de nota o acción…"
      @input="activeIndex = 0"
      @keydown="onKey"
    />
    <ul id="palette-list" class="palette-list" role="listbox" aria-label="Resultados">
      <li
        v-for="(item, i) in items"
        :id="`palette-${i}`"
        :key="item.kind + item.id"
        role="option"
        :aria-selected="i === activeIndex"
        class="palette-item"
        @click="choose(item)"
        @mousemove="activeIndex = i"
      >
        <Zap v-if="item.kind !== 'file'" :size="14" aria-hidden="true" />
        <FileText v-else :size="14" aria-hidden="true" />
        <span class="palette-label">{{ item.label }}</span>
        <kbd v-if="item.hint">{{ item.hint }}</kbd>
      </li>
      <li v-if="!items.length" class="palette-empty">Sin coincidencias</li>
    </ul>
  </Modal>
</template>

<style scoped>
.palette-input {
  min-height: 2.4rem;
  font-size: 15px;
}
.palette-list {
  list-style: none;
  margin: 10px 0 0;
  padding: 0;
  max-height: 50vh;
  overflow: auto;
}
.palette-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  color: var(--muted);
}
.palette-item[aria-selected="true"] {
  background: var(--primary-soft);
  color: var(--fg);
}
.palette-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--fg);
}
.palette-empty {
  padding: 16px;
  text-align: center;
  color: var(--muted);
}
</style>
