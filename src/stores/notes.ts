// Sustituye a concept.store y folder.store de cogni-vault. Habla con el backend
// Rust a través de `api` (IPC de Tauri) en lugar de fetch a /api/*.
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, type CreateNote, type Folder, type Note, type NoteSummary, type Tag } from "../lib/api";
import type { FileItem, FolderItem } from "../types";

export const useNotesStore = defineStore("notes", () => {
  const folders = ref<Folder[]>([]);
  const summaries = ref<NoteSummary[]>([]);
  /** Notas completas (con cuerpo) ya cargadas. Es la fuente de verdad del editor. */
  const notes = ref<Record<string, Note>>({});
  const tags = ref<Tag[]>([]);
  /** Etiquetas de cada nota, cargadas bajo demanda (ver `loadNoteTags`). */
  const noteTags = ref<Record<string, Tag[]>>({});
  const loading = ref(false);
  const loadError = ref("");

  const folderItems = computed<FolderItem[]>(() => folders.value.map((f) => ({ id: f.id, name: f.name, parentId: f.parentId, version: f.version })));
  const files = computed<FileItem[]>(() =>
    summaries.value.map((s) => {
      const full = notes.value[s.id];
      return {
        id: s.id,
        name: full?.title ?? s.title,
        folderId: full?.folderId ?? s.folderId,
        content: full?.body ?? "",
        updatedAt: Date.parse(full?.updatedAt ?? s.updatedAt),
      };
    }),
  );

  function upsertSummary(n: NoteSummary | Note) {
    const summary: NoteSummary = {
      id: n.id,
      folderId: n.folderId,
      title: n.title,
      createdAt: n.createdAt,
      updatedAt: n.updatedAt,
    };
    const i = summaries.value.findIndex((s) => s.id === n.id);
    if (i < 0) summaries.value.push(summary);
    else summaries.value[i] = summary;
  }

  function cache(note: Note): Note {
    notes.value[note.id] = note;
    upsertSummary(note);
    return notes.value[note.id];
  }

  function forget(id: string) {
    delete notes.value[id];
    delete noteTags.value[id];
    summaries.value = summaries.value.filter((s) => s.id !== id);
  }

  async function loadAll() {
    loading.value = true;
    loadError.value = "";
    try {
      const [f, s, t] = await Promise.all([api.listFolders(), api.listNotes(), api.listTags()]);
      folders.value = f;
      summaries.value = s;
      tags.value = t;
      // Descartar de la caché las notas que ya no existen (papelera, purga…).
      const alive = new Set(s.map((n) => n.id));
      for (const id of Object.keys(notes.value)) if (!alive.has(id)) delete notes.value[id];
    } catch (e) {
      loadError.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function refreshFolders() {
    folders.value = await api.listFolders();
  }

  // ---------- Notas ----------

  async function ensureNote(id: string): Promise<Note> {
    return notes.value[id] ?? cache(await api.getNote(id));
  }

  async function createNote(input: CreateNote) {
    return cache(await api.createNote(input));
  }

  /**
   * Guarda título y cuerpo con control de versión optimista. Solo se aplican la
   * versión y la fecha devueltas: lo que se haya escrito mientras tanto se respeta.
   */
  async function saveNote(id: string) {
    const current = notes.value[id];
    if (!current) return;
    const updated = await api.updateNote(id, {
      // El backend rechaza títulos vacíos; mientras se escribe, se guarda uno provisional.
      title: current.title.trim() || "Sin título",
      body: current.body,
      expectedVersion: current.version,
    });
    current.version = updated.version;
    current.updatedAt = updated.updatedAt;
    upsertSummary({ ...updated, title: current.title || updated.title });
  }

  /** Tras un CONFLICT: adopta la versión del disco para que el siguiente guardado la sobrescriba. */
  async function adoptServerVersion(id: string) {
    const server = await api.getNote(id);
    if (notes.value[id]) notes.value[id].version = server.version;
  }

  /** Tras un CONFLICT: descarta lo local y recarga la nota del disco. */
  async function reloadNote(id: string) {
    return cache(await api.getNote(id));
  }

  async function moveNote(id: string, folderId: string | null) {
    const updated = await api.moveNote(id, folderId);
    const current = notes.value[id];
    // Mover incrementa la versión en el backend: hay que reflejarlo o el siguiente guardado dará CONFLICT.
    if (current) Object.assign(current, { folderId: updated.folderId, version: updated.version, updatedAt: updated.updatedAt });
    upsertSummary(current ?? updated);
  }

  async function trashNote(id: string) {
    await api.trashNote(id);
    forget(id);
  }

  async function restoreVersion(id: string, versionId: string) {
    return cache(await api.restoreNoteVersion(id, versionId));
  }

  // ---------- Carpetas ----------

  async function createFolder(name: string, parentId: string | null) {
    const folder = await api.createFolder(name, parentId);
    await refreshFolders();
    return folder;
  }

  async function renameFolder(id: string, name: string) {
    await api.renameFolder(id, name);
    await refreshFolders();
  }

  async function moveFolder(id: string, targetParentId: string | null) {
    const folder = folders.value.find((f) => f.id === id);
    if (!folder) return;
    await api.moveFolder(id, targetParentId, folder.version);
    await refreshFolders();
  }

  async function trashFolder(id: string) {
    const result = await api.trashFolder(id);
    await loadAll();
    return result;
  }

  // ---------- Etiquetas ----------

  async function loadTags() {
    tags.value = await api.listTags();
  }

  async function loadNoteTags(id: string) {
    noteTags.value[id] = await api.noteTags(id);
  }

  /** Reemplaza las etiquetas de una nota y refresca los contadores globales. */
  async function setNoteTags(id: string, names: string[]) {
    noteTags.value[id] = await api.setNoteTags(id, names);
    await loadTags();
  }

  async function deleteTag(id: string) {
    await api.deleteTag(id);
    tags.value = tags.value.filter((t) => t.id !== id);
    for (const key of Object.keys(noteTags.value)) noteTags.value[key] = noteTags.value[key].filter((t) => t.id !== id);
  }

  function notesByTag(tagId: string) {
    return api.listNotes(tagId);
  }

  return {
    folders,
    summaries,
    notes,
    tags,
    noteTags,
    loading,
    loadError,
    folderItems,
    files,
    loadAll,
    refreshFolders,
    ensureNote,
    createNote,
    saveNote,
    adoptServerVersion,
    reloadNote,
    moveNote,
    trashNote,
    restoreVersion,
    createFolder,
    renameFolder,
    moveFolder,
    trashFolder,
    loadTags,
    loadNoteTags,
    setNoteTags,
    deleteTag,
    notesByTag,
  };
});
