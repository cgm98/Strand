<script setup lang="ts">
// Portado de cogni-vault. Cambio: usa los comandos IPC y el diálogo común.
import { onMounted, ref } from "vue";
import Modal from "./Modal.vue";
import { api, ApiError, type Note, type NoteVersion, type NoteVersionSummary } from "../lib/api";
import { useNotesStore } from "../stores/notes";

const props = defineProps<{ noteId: string }>();
const emit = defineEmits<{ (e: "close"): void; (e: "restored", note: Note): void }>();
const store = useNotesStore();

const versions = ref<NoteVersionSummary[]>([]);
const selected = ref<NoteVersion | null>(null);
const loading = ref(true);
const busy = ref(false);
const error = ref("");
const confirmRestore = ref(false);

const message = (e: unknown) => (e instanceof ApiError ? e.message : String(e));
const when = (iso: string) => new Date(iso).toLocaleString("es", { dateStyle: "medium", timeStyle: "short" });

async function load() {
  loading.value = true;
  error.value = "";
  try {
    versions.value = await api.listNoteVersions(props.noteId);
  } catch (e) {
    error.value = message(e);
  } finally {
    loading.value = false;
  }
}

async function preview(id: string) {
  busy.value = true;
  error.value = "";
  confirmRestore.value = false;
  try {
    selected.value = await api.getNoteVersion(props.noteId, id);
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function restore() {
  if (!selected.value || busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    emit("restored", await store.restoreVersion(props.noteId, selected.value.id));
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

onMounted(load);
</script>

<template>
  <Modal title="Historial de versiones" size="lg" @close="!busy && emit('close')">
    <p v-if="loading" role="status" class="muted">Cargando versiones…</p>
    <div v-else-if="error" role="alert" class="error">{{ error }} <button type="button" class="btn" @click="load">Reintentar</button></div>
    <p v-else-if="!versions.length" class="muted">
      Esta nota todavía no tiene versiones guardadas. Se guarda una instantánea como máximo cada 15 minutos de edición.
    </p>
    <div v-if="versions.length" class="history">
      <ul class="history-list" aria-label="Versiones">
        <li v-for="version in versions" :key="version.id">
          <button type="button" :aria-current="selected?.id === version.id ? 'true' : undefined" :disabled="busy" @click="preview(version.id)">
            {{ when(version.createdAt) }}
          </button>
        </li>
      </ul>
      <div class="history-preview">
        <template v-if="selected">
          <h3>{{ selected.title }}</h3>
          <pre>{{ selected.body || "(Nota vacía)" }}</pre>
        </template>
        <p v-else class="muted">Elige una versión para ver su contenido.</p>
      </div>
    </div>
    <template v-if="selected" #footer>
      <template v-if="!confirmRestore">
        <button type="button" class="btn primary" :disabled="busy" @click="confirmRestore = true">Restaurar esta versión</button>
      </template>
      <template v-else>
        <p class="confirm-text">Se reemplazarán el título y el contenido actuales. El estado actual se guardará como otra versión.</p>
        <button type="button" class="btn" :disabled="busy" @click="confirmRestore = false">Cancelar</button>
        <button type="button" class="btn primary" :disabled="busy" @click="restore">{{ busy ? "Restaurando…" : "Confirmar restauración" }}</button>
      </template>
    </template>
  </Modal>
</template>

<style scoped>
.muted {
  color: var(--muted);
  margin: 0;
}
.error {
  color: var(--danger);
  display: flex;
  gap: 12px;
  align-items: center;
}
.history {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: 16px;
}
.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 60vh;
  overflow: auto;
}
.history-list button {
  width: 100%;
  text-align: left;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12px;
}
.history-list button:hover:not(:disabled) {
  background: var(--surface);
}
.history-list button[aria-current] {
  border-color: var(--primary);
  background: var(--primary-soft);
}
.history-preview {
  min-width: 0;
}
.history-preview h3 {
  margin: 0 0 8px;
  font-size: 15px;
}
.history-preview pre {
  margin: 0;
  padding: 14px;
  max-height: 55vh;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: 12px/1.7 var(--font-mono);
  background: var(--surface);
  border-radius: 8px;
}
.confirm-text {
  flex-basis: 100%;
  margin: 0;
  font-size: 13px;
  color: var(--muted);
}
@media (max-width: 640px) {
  .history {
    grid-template-columns: 1fr;
  }
}
</style>
