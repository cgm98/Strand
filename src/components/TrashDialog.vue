<script setup lang="ts">
// Papelera: notas sueltas y carpetas enviadas con todo su contenido (lotes).
// Restaurar es seguro; purgar y vaciar piden confirmación porque no se deshacen.
import { onMounted, ref } from "vue";
import Modal from "./Modal.vue";
import { api, ApiError, type TrashContents } from "../lib/api";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";

const emit = defineEmits<{ (e: "close"): void; (e: "changed"): void }>();
const { confirm } = useConfirm();
const toast = useToast();

const trash = ref<TrashContents>({ notes: [], batches: [] });
const loading = ref(true);
const busy = ref(false);
const when = (iso: string) => new Date(iso).toLocaleString("es", { dateStyle: "medium", timeStyle: "short" });

async function load() {
  loading.value = true;
  try {
    trash.value = await api.listTrash();
  } catch (e) {
    toast.error(e instanceof ApiError ? e.message : String(e));
  } finally {
    loading.value = false;
  }
}

async function act(fn: () => Promise<unknown>, done: string) {
  busy.value = true;
  try {
    await fn();
    toast.success(done);
    emit("changed");
    await load();
  } catch (e) {
    toast.error(e instanceof ApiError ? e.message : String(e));
  } finally {
    busy.value = false;
  }
}

async function restoreBatch(id: string, name: string) {
  busy.value = true;
  try {
    const r = await api.restoreTrashBatch(id);
    const notes: string[] = [];
    if (r.renamed) notes.push(`${r.renamed} renombrada(s) por coincidir con otra`);
    if (r.reparented) notes.push(`${r.reparented} movida(s) a la raíz porque su carpeta ya no existe`);
    toast.success(`«${name}» restaurada: ${r.folders} carpetas y ${r.notes} notas${notes.length ? ". " + notes.join("; ") : ""}`);
    emit("changed");
    await load();
  } catch (e) {
    toast.error(e instanceof ApiError ? e.message : String(e));
  } finally {
    busy.value = false;
  }
}

async function purge(kind: "note" | "batch", id: string, name: string) {
  const ok = await confirm({
    title: "Borrar definitivamente",
    message: `«${name}» se borrará para siempre${kind === "batch" ? " junto con todo su contenido" : ""}. No se puede deshacer.`,
    confirmLabel: "Borrar para siempre",
    destructive: true,
  });
  if (ok) await act(() => (kind === "note" ? api.purgeNote(id) : api.purgeTrashBatch(id)), `«${name}» borrada definitivamente`);
}

async function emptyAll() {
  const ok = await confirm({
    title: "Vaciar la papelera",
    message: "Todo lo que hay en la papelera se borrará para siempre. No se puede deshacer.",
    confirmLabel: "Vaciar papelera",
    destructive: true,
  });
  if (ok) await act(() => api.emptyTrash(), "Papelera vaciada");
}

onMounted(load);
</script>

<template>
  <Modal title="Papelera" size="md" @close="emit('close')">
    <p v-if="loading" role="status" class="muted">Cargando…</p>
    <p v-else-if="!trash.notes.length && !trash.batches.length" class="muted">La papelera está vacía.</p>
    <template v-else>
      <section v-if="trash.batches.length" aria-labelledby="trash-folders">
        <h3 id="trash-folders" class="trash-heading">Carpetas</h3>
        <ul class="trash-list">
          <li v-for="b in trash.batches" :key="b.id" class="trash-item">
            <div class="trash-info">
              <span class="trash-name">{{ b.rootName }}</span>
              <span class="muted small">{{ b.itemCount }} elementos · {{ when(b.createdAt) }}</span>
            </div>
            <button type="button" class="btn" :disabled="busy" @click="restoreBatch(b.id, b.rootName)">Restaurar</button>
            <button type="button" class="btn danger" :disabled="busy" @click="purge('batch', b.id, b.rootName)">Borrar</button>
          </li>
        </ul>
      </section>
      <section v-if="trash.notes.length" aria-labelledby="trash-notes">
        <h3 id="trash-notes" class="trash-heading">Notas</h3>
        <ul class="trash-list">
          <li v-for="n in trash.notes" :key="n.id" class="trash-item">
            <div class="trash-info">
              <span class="trash-name">{{ n.title }}</span>
              <span class="muted small">Modificada {{ when(n.updatedAt) }}</span>
            </div>
            <button type="button" class="btn" :disabled="busy" @click="act(() => api.restoreNote(n.id), `«${n.title}» restaurada`)">Restaurar</button>
            <button type="button" class="btn danger" :disabled="busy" @click="purge('note', n.id, n.title)">Borrar</button>
          </li>
        </ul>
      </section>
    </template>
    <template v-if="trash.notes.length || trash.batches.length" #footer>
      <button type="button" class="btn danger" :disabled="busy" @click="emptyAll">Vaciar papelera…</button>
    </template>
  </Modal>
</template>

<style scoped>
.muted {
  color: var(--muted);
  margin: 0;
}
.small {
  font-size: 12px;
}
.trash-heading {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  margin: 0 0 6px;
}
section + section {
  margin-top: 18px;
}
.trash-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.trash-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}
.trash-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.trash-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
