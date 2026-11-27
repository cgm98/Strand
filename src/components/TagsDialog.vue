<script setup lang="ts">
// Lista todas las etiquetas con su número de notas; al abrir una se cargan (y
// cachean) las notas que la usan. Borrar una etiqueta la quita de todas sus notas.
import { onMounted, ref } from "vue";
import { ChevronRight } from "lucide-vue-next";
import Modal from "./Modal.vue";
import { ApiError, type NoteSummary, type Tag } from "../lib/api";
import { useNotesStore } from "../stores/notes";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";

const emit = defineEmits<{ (e: "close"): void; (e: "selectFile", id: string): void }>();
const store = useNotesStore();
const { confirm } = useConfirm();
const toast = useToast();
const message = (e: unknown) => (e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e));

const loading = ref(true);
const openTagId = ref<string | null>(null);
const notesByTag = ref<Record<string, NoteSummary[]>>({});
const notesLoading = ref<string | null>(null);

onMounted(async () => {
  try {
    await store.loadTags();
  } catch (e) {
    toast.error(message(e));
  } finally {
    loading.value = false;
  }
});

async function toggle(tagId: string) {
  if (openTagId.value === tagId) {
    openTagId.value = null;
    return;
  }
  openTagId.value = tagId;
  if (notesByTag.value[tagId]) return;
  notesLoading.value = tagId;
  try {
    notesByTag.value[tagId] = await store.notesByTag(tagId);
  } catch (e) {
    toast.error(message(e));
  } finally {
    notesLoading.value = null;
  }
}

function choose(id: string) {
  emit("selectFile", id);
  emit("close");
}

async function remove(tag: Tag) {
  const ok = await confirm({
    title: "Borrar etiqueta",
    message: tag.noteCount ? `«${tag.name}» se quitará de ${tag.noteCount} nota(s). No se puede deshacer.` : `«${tag.name}» se borrará.`,
    confirmLabel: "Borrar etiqueta",
    destructive: true,
  });
  if (!ok) return;
  try {
    await store.deleteTag(tag.id);
    delete notesByTag.value[tag.id];
    if (openTagId.value === tag.id) openTagId.value = null;
    toast.success(`«${tag.name}» borrada`);
  } catch (e) {
    toast.error(message(e));
  }
}
</script>

<template>
  <Modal title="Etiquetas" size="md" @close="emit('close')">
    <p v-if="loading" role="status" class="muted">Cargando…</p>
    <p v-else-if="!store.tags.length" class="muted">Todavía no hay etiquetas. Añade una desde las propiedades de una nota.</p>
    <ul v-else class="tags-list">
      <li v-for="tag in store.tags" :key="tag.id" class="tags-item">
        <div class="tags-row">
          <button type="button" class="tags-toggle" :aria-expanded="openTagId === tag.id" @click="toggle(tag.id)">
            <ChevronRight :size="14" class="tags-chevron" :class="{ open: openTagId === tag.id }" aria-hidden="true" />
            <span class="tags-name">#{{ tag.name }}</span>
            <small class="muted">{{ tag.noteCount }} nota(s)</small>
          </button>
          <button type="button" class="btn danger" :aria-label="`Borrar etiqueta ${tag.name}`" @click="remove(tag)">Borrar</button>
        </div>
        <ul v-if="openTagId === tag.id" class="tags-notes">
          <li v-if="notesLoading === tag.id" class="muted">Cargando notas…</li>
          <li v-else-if="!notesByTag[tag.id]?.length" class="muted">Sin notas.</li>
          <li v-for="note in notesByTag[tag.id]" :key="note.id">
            <button type="button" class="tags-note" @click="choose(note.id)">{{ note.title }}</button>
          </li>
        </ul>
      </li>
    </ul>
  </Modal>
</template>

<style scoped>
.muted {
  color: var(--muted);
  margin: 0;
}
.tags-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.tags-item {
  border-bottom: 1px solid var(--border);
  padding: 4px 0;
}
.tags-item:last-child {
  border-bottom: 0;
}
.tags-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.tags-toggle {
  display: flex;
  flex: 1;
  min-width: 0;
  align-items: center;
  gap: 8px;
  padding: 8px 4px;
  text-align: left;
  border-radius: 5px;
}
.tags-toggle:hover {
  background: var(--surface-2);
}
.tags-chevron {
  flex-shrink: 0;
  transition: transform 0.15s;
  color: var(--muted);
}
.tags-chevron.open {
  transform: rotate(90deg);
}
.tags-name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
@media (prefers-reduced-motion: reduce) {
  .tags-chevron {
    transition: none;
  }
}
.tags-notes {
  list-style: none;
  margin: 2px 0 8px 22px;
  padding: 0;
}
.tags-note {
  display: block;
  width: 100%;
  padding: 6px 8px;
  text-align: left;
  border-radius: 5px;
  color: var(--muted);
}
.tags-note:hover {
  background: var(--surface-2);
  color: var(--fg);
}
</style>
