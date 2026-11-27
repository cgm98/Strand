<script setup lang="ts">
// Portado de cogni-vault (Sidebar/Index.vue). Cambios: sin espacios/cerebros
// (todo son carpetas), raíz = null, accesibilidad del árbol y textos.
import { computed, nextTick, ref, watch } from "vue";
import { ChevronRight, ChevronsDownUp, FileText, Folder, FolderOpen, FolderPlus, MoreHorizontal, Plus, Search } from "lucide-vue-next";
import { folderPath, isWithin, treeRows, type TreeRow } from "../lib/noteTree";
import type { FileItem, FolderItem } from "../types";
import { useContextMenu, type MenuEntry } from "../composables/useContextMenu";
import { useConfirm } from "../composables/useConfirm";

const props = defineProps<{
  files: FileItem[];
  folders: FolderItem[];
  activeFileId: string | null;
  loading?: boolean;
  error?: string;
  creating?: boolean;
}>();

const emit = defineEmits<{
  (e: "selectFile", id: string): void;
  (e: "createFile", folderId: string | null): void;
  (e: "createFolder", parentId: string | null, name: string): void;
  (e: "renameFolder", id: string, name: string): void;
  (e: "moveFolder", id: string, target: string | null): void;
  (e: "deleteFolder", id: string): void;
  (e: "importMarkdownFiles", folderId: string | null, files: File[]): void;
  (e: "moveNote", id: string, folderId: string | null): void;
  (e: "trashNote", id: string): void;
}>();
const menu = useContextMenu();
const { confirm } = useConfirm();

const filter = ref("");
const filterInput = ref<HTMLInputElement>();
const expanded = ref(new Set<string>());
const selectedFolder = ref<string | null>(null);
const importInput = ref<HTMLInputElement>();

const selectedLabel = computed(() => folderPath(props.folders, selectedFolder.value).join(" / ") || "Raíz");
const rows = computed(() => treeRows(props.folders, props.files, expanded.value, filter.value));
// Si la carpeta seleccionada desaparece (papelera), volver a la raíz.
watch(
  () => props.folders,
  (folders) => {
    if (selectedFolder.value && !folders.some((f) => f.id === selectedFolder.value)) selectedFolder.value = null;
  },
);
// Expandir los antepasados de la nota activa para que se vea en el árbol.
watch(
  () => [props.activeFileId, props.files, props.folders] as const,
  () => {
    let id = props.files.find((f) => f.id === props.activeFileId)?.folderId ?? null;
    const seen = new Set<string>();
    while (id && !seen.has(id)) {
      seen.add(id);
      expanded.value.add(id);
      id = props.folders.find((f) => f.id === id)?.parentId ?? null;
    }
  },
  { immediate: true },
);

function toggle(folder: FolderItem) {
  selectedFolder.value = folder.id;
  if (expanded.value.has(folder.id)) expanded.value.delete(folder.id);
  else expanded.value.add(folder.id);
}

// ---------- Crear y renombrar en el propio árbol ----------
// Como en VS Code u Obsidian: sin paneles aparte. Una fila del árbol se vuelve
// un campo de texto; Intro guarda, Esc cancela y salir del campo guarda si el
// nombre es válido.
type Draft = { mode: "create"; parentId: string | null } | { mode: "rename"; folder: FolderItem };
const draft = ref<Draft | null>(null);
const draftName = ref("");
const draftInput = ref<HTMLInputElement | null>(null);
// Dentro de v-for, `ref="x"` daría un array: se usa una función.
const setDraftInput = (el: unknown) => void (draftInput.value = el as HTMLInputElement | null);
const DRAFT_ID = "__new-folder__";

const draftParent = computed(() => (draft.value?.mode === "create" ? draft.value.parentId : (draft.value?.folder.parentId ?? null)));
/** Mismo criterio que la base de datos: el nombre es único entre hermanos sin distinguir mayúsculas. */
const draftError = computed(() => {
  const name = draftName.value.trim().toLocaleLowerCase();
  if (!draft.value || !name) return "";
  const self = draft.value.mode === "rename" ? draft.value.folder.id : null;
  const clash = props.folders.find((f) => f.id !== self && (f.parentId ?? null) === draftParent.value && f.name.toLocaleLowerCase() === name);
  return clash ? `Ya hay una carpeta «${clash.name}» aquí` : "";
});

/** Filas del árbol con la carpeta nueva en borrador como primer hijo de su carpeta. */
const displayRows = computed<TreeRow[]>(() => {
  const d = draft.value;
  if (d?.mode !== "create") return rows.value;
  const draftRow: TreeRow = { id: DRAFT_ID, name: "", depth: 0 };
  if (!d.parentId) return [draftRow, ...rows.value];
  const index = rows.value.findIndex((r) => r.id === d.parentId);
  if (index < 0) return [draftRow, ...rows.value];
  return [...rows.value.slice(0, index + 1), { ...draftRow, depth: rows.value[index].depth + 1 }, ...rows.value.slice(index + 1)];
});

async function startDraft(next: Draft) {
  filter.value = "";
  draft.value = next;
  draftName.value = next.mode === "rename" ? next.folder.name : "";
  if (next.mode === "create" && next.parentId) expanded.value.add(next.parentId);
  await nextTick();
  draftInput.value?.focus();
  draftInput.value?.select();
}

const startCreate = (parentId: string | null) => startDraft({ mode: "create", parentId });
const startRename = (folder: FolderItem) => startDraft({ mode: "rename", folder });

function focusFolder(id: string | null) {
  void nextTick(() => {
    const el = id && document.querySelector<HTMLElement>(`[data-folder-id="${CSS.escape(id)}"]`);
    if (el) el.focus();
  });
}

function cancelDraft() {
  const d = draft.value;
  draft.value = null;
  focusFolder(d?.mode === "rename" ? d.folder.id : (d?.parentId ?? null));
}

/** `fromBlur`: al salir del campo, un nombre inválido cancela en vez de retener el foco. */
function commitDraft(fromBlur = false) {
  const d = draft.value;
  if (!d) return; // ya se guardó con Intro y el campo desapareció
  const name = draftName.value.trim();
  if (draftError.value) {
    if (fromBlur) cancelDraft();
    return;
  }
  if (!name || (d.mode === "rename" && name === d.folder.name)) return cancelDraft();
  draft.value = null;
  if (d.mode === "create") {
    emit("createFolder", d.parentId, name);
    if (d.parentId) expanded.value.add(d.parentId);
    focusFolder(d.parentId);
  } else {
    emit("renameFolder", d.folder.id, name);
    focusFolder(d.folder.id);
  }
}

function onFolderKey(event: KeyboardEvent, folder: FolderItem) {
  if (event.key === "F2") {
    event.preventDefault();
    void startRename(folder);
  } else if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
    event.preventDefault();
    openFolderMenuAt(event.currentTarget as HTMLElement, folder);
  }
}

// ---------- Menús contextuales ----------
const folderLabel = (id: string) => folderPath(props.folders, id).join(" / ");
const sortedFolders = computed(() => props.folders.map((f) => ({ id: f.id, label: folderLabel(f.id) })).sort((a, b) => a.label.localeCompare(b.label, "es")));

function noteMenu(event: MouseEvent, file: FileItem) {
  menu.open(event, [
    { label: "Abrir", action: () => emit("selectFile", file.id) },
    {
      label: "Mover a",
      children: [
        { label: "Raíz", checked: !file.folderId, action: () => emit("moveNote", file.id, null) },
        ...sortedFolders.value.map((f) => ({ label: f.label, checked: file.folderId === f.id, action: () => emit("moveNote", file.id, f.id) })),
      ],
    },
    { label: "Copiar enlace [[…]]", action: () => navigator.clipboard.writeText(`[[${file.name}]]`) },
    "separator",
    { label: "Mover a la papelera", danger: true, action: () => emit("trashNote", file.id) },
  ]);
}

function folderMenuItems(folder: FolderItem): MenuEntry[] {
  const targets: MenuEntry[] = [
    { label: "Raíz", checked: !folder.parentId, disabled: !folder.parentId, action: () => emit("moveFolder", folder.id, null) },
    ...sortedFolders.value
      .filter((f) => !isWithin(props.folders, f.id, folder.id))
      .map((f) => ({
        label: f.label,
        checked: folder.parentId === f.id,
        disabled: folder.parentId === f.id,
        action: () => emit("moveFolder", folder.id, f.id),
      })),
  ];
  return [
    { label: "Nueva nota aquí", action: () => emit("createFile", folder.id) },
    { label: "Nueva subcarpeta", action: () => startCreate(folder.id) },
    "separator",
    { label: "Renombrar", shortcut: "F2", action: () => startRename(folder) },
    { label: "Mover a", children: targets },
    { label: "Importar archivos .md…", action: () => importInput.value?.click() },
    "separator",
    {
      label: "Mover a la papelera…",
      danger: true,
      action: async () => {
        const ok = await confirm({
          title: "Mover carpeta a la papelera",
          message: `«${folder.name}», sus subcarpetas y sus notas irán a la papelera. Podrás restaurarlas desde allí.`,
          confirmLabel: "Mover a la papelera",
          destructive: true,
        });
        if (ok) emit("deleteFolder", folder.id);
      },
    },
  ];
}

function folderMenu(event: MouseEvent, folder: FolderItem) {
  selectedFolder.value = folder.id;
  menu.open(event, folderMenuItems(folder));
}

function openFolderMenuAt(anchor: HTMLElement, folder: FolderItem) {
  selectedFolder.value = folder.id;
  const rect = anchor.getBoundingClientRect();
  menu.openAt(Math.max(8, rect.right - 220), rect.bottom + 4, folderMenuItems(folder));
}

function backgroundMenu(event: MouseEvent) {
  if ((event.target as HTMLElement).closest(".tree-entry")) return;
  selectedFolder.value = null;
  menu.open(event, [
    { label: "Nueva nota", shortcut: "Ctrl+N", action: () => emit("createFile", null) },
    { label: "Nueva carpeta", action: () => startCreate(null) },
    "separator",
    { label: "Contraer todas las carpetas", action: () => expanded.value.clear() },
  ]);
}

function importFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  emit("importMarkdownFiles", selectedFolder.value, Array.from(input.files ?? []));
  input.value = "";
}

function focusFilter() {
  filterInput.value?.focus();
  filterInput.value?.select();
}
defineExpose({ focusFilter });
</script>

<template>
  <aside class="file-explorer" aria-label="Explorador de notas">
    <header class="tree-toolbar">
      <button type="button" class="tree-heading" :aria-pressed="selectedFolder === null" title="Seleccionar la raíz" @click="selectedFolder = null">
        Notas
      </button>
      <button type="button" class="icon-btn" aria-label="Contraer todas las carpetas" title="Contraer carpetas" @click="expanded.clear()">
        <ChevronsDownUp :size="15" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="icon-btn"
        :aria-label="`Nueva carpeta en ${selectedLabel}`"
        title="Nueva carpeta"
        :disabled="loading || !!error"
        @click="startCreate(selectedFolder)"
      >
        <FolderPlus :size="16" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="icon-btn"
        :aria-label="`Nueva nota en ${selectedLabel}`"
        title="Nueva nota (Ctrl+N)"
        :disabled="loading || !!error || creating"
        @click="emit('createFile', selectedFolder)"
      >
        <Plus :size="17" aria-hidden="true" />
      </button>
    </header>

    <input ref="importInput" type="file" accept=".md,.markdown,text/markdown" multiple class="sr-only" tabindex="-1" aria-hidden="true" @change="importFiles" />

    <div class="tree-search">
      <Search :size="14" aria-hidden="true" />
      <input
        ref="filterInput"
        v-model="filter"
        type="search"
        placeholder="Filtrar por título…"
        aria-label="Filtrar notas y carpetas por título"
        @keydown.esc="filter = ''"
      />
    </div>

    <nav class="tree-scroll" aria-label="Carpetas y notas" @contextmenu="backgroundMenu">
      <p v-if="loading" class="tree-empty" role="status">Cargando…</p>
      <p v-else-if="error" class="tree-empty" role="alert">No se pudieron cargar las notas: {{ error }}</p>
      <template v-else>
        <ul class="tree-list">
          <li
            v-for="row in displayRows"
            :key="(row.folder ? 'folder:' : 'file:') + row.id"
            class="tree-row"
            :style="{ paddingLeft: 6 + row.depth * 14 + 'px' }"
          >
            <!-- Carpeta en borrador (nueva) o en renombrado: la fila es el campo. -->
            <div
              v-if="row.id === DRAFT_ID || (draft?.mode === 'rename' && draft.folder.id === row.id)"
              class="tree-entry tree-folder tree-editing"
              :class="{ invalid: !!draftError }"
            >
              <ChevronRight :size="12" class="tree-chevron" :class="{ open: row.folder && expanded.has(row.id) }" aria-hidden="true" />
              <FolderOpen v-if="row.folder && expanded.has(row.id)" :size="15" class="tree-folder-icon" aria-hidden="true" />
              <Folder v-else :size="15" class="tree-folder-icon" aria-hidden="true" />
              <input
                :ref="setDraftInput"
                v-model="draftName"
                class="tree-input"
                maxlength="150"
                spellcheck="false"
                :placeholder="draft?.mode === 'create' ? 'Nombre de la carpeta' : ''"
                :aria-label="
                  draft?.mode === 'create'
                    ? `Nombre de la nueva carpeta en ${folderPath(folders, draftParent).join(' / ') || 'la raíz'}`
                    : `Nuevo nombre de «${row.name}»`
                "
                :aria-invalid="!!draftError"
                :aria-describedby="draftError ? 'tree-draft-error' : 'tree-draft-hint'"
                @keydown.enter.prevent="commitDraft()"
                @keydown.esc.prevent.stop="cancelDraft"
                @blur="commitDraft(true)"
              />
            </div>
            <template v-else-if="row.folder">
              <button
                type="button"
                class="tree-entry tree-folder"
                :class="{ selected: selectedFolder === row.id }"
                :data-folder-id="row.id"
                :aria-expanded="!!filter.trim() || expanded.has(row.id)"
                :title="row.name"
                @click="toggle(row.folder)"
                @dblclick.prevent="startRename(row.folder)"
                @keydown="onFolderKey($event, row.folder)"
                @contextmenu="folderMenu($event, row.folder)"
              >
                <ChevronRight :size="12" class="tree-chevron" :class="{ open: !!filter.trim() || expanded.has(row.id) }" aria-hidden="true" />
                <component :is="expanded.has(row.id) || filter.trim() ? FolderOpen : Folder" :size="15" class="tree-folder-icon" aria-hidden="true" />
                <span class="tree-label">{{ row.name }}</span>
                <small class="tree-count" :aria-label="`${row.count} notas`">{{ row.count }}</small>
              </button>
              <button
                type="button"
                class="tree-more"
                aria-haspopup="menu"
                :aria-label="`Opciones de «${row.name}»`"
                title="Opciones de la carpeta"
                @click="openFolderMenuAt($event.currentTarget as HTMLElement, row.folder)"
              >
                <MoreHorizontal :size="15" aria-hidden="true" />
              </button>
            </template>
            <button
              v-else
              type="button"
              class="tree-entry tree-file"
              :class="{ active: activeFileId === row.id }"
              :aria-current="activeFileId === row.id ? 'page' : undefined"
              :title="row.name"
              @click="emit('selectFile', row.id)"
              @contextmenu="
                noteMenu(
                  $event,
                  files.find((f) => f.id === row.id)!,
                )
              "
            >
              <FileText :size="14" aria-hidden="true" />
              <span class="tree-label">{{ row.name || "Sin título" }}</span>
            </button>
            <p
              v-if="draftError && (row.id === DRAFT_ID || (draft?.mode === 'rename' && draft.folder.id === row.id))"
              id="tree-draft-error"
              class="tree-draft-message"
              role="alert"
            >
              {{ draftError }}
            </p>
          </li>
        </ul>
        <p id="tree-draft-hint" class="sr-only">Intro para guardar, Escape para cancelar.</p>
        <p v-if="!displayRows.length" class="tree-empty">{{ filter ? "Sin coincidencias" : "Aún no hay notas. Crea la primera con el botón +." }}</p>
      </template>
    </nav>

    <footer class="tree-footer">{{ selectedLabel }} · {{ files.length }} notas</footer>
  </aside>
</template>

<style scoped>
.file-explorer {
  width: 280px;
  min-width: 220px;
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-right: 1px solid var(--border);
  font-size: 13px;
}
.tree-toolbar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 8px;
  border-bottom: 1px solid var(--border);
}
.tree-heading {
  margin-right: auto;
  padding: 4px 6px;
  border-radius: 5px;
  text-transform: uppercase;
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.08em;
  color: var(--muted);
}
.tree-heading:hover {
  color: var(--fg);
}
.tree-search {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 10px;
  padding: 5px 8px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--muted);
}
.tree-search:focus-within {
  border-color: var(--focus);
}
.tree-search input {
  width: 100%;
  min-width: 0;
  background: transparent;
  border: 0;
  outline: none;
  color: var(--fg);
}
.tree-scroll {
  flex: 1;
  overflow: auto;
  padding: 0 6px 12px;
}
.tree-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.tree-entry {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 30px;
  padding: 4px 6px;
  text-align: left;
  border-radius: 5px;
  min-width: 0;
}
.tree-entry svg {
  flex-shrink: 0;
}
.tree-entry:hover {
  background: var(--surface-2);
}
.tree-row {
  position: relative;
}

/* «⋯» de cada carpeta: aparece al pasar el ratón o con el foco, en el sitio del contador. */
.tree-more {
  position: absolute;
  right: 4px;
  top: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 22px;
  border-radius: 4px;
  color: var(--muted);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.12s;
}
.tree-row:hover .tree-more,
.tree-more:focus-visible {
  opacity: 1;
  pointer-events: auto;
}
.tree-more:hover {
  background: var(--border);
  color: var(--fg);
}
.tree-row:hover .tree-count,
.tree-row:has(.tree-more:focus-visible) .tree-count {
  visibility: hidden;
}

/* Edición en línea: la fila conserva icono y sangría; solo el nombre se vuelve campo. */
.tree-editing {
  background: var(--surface-2);
  animation: tree-row-in 0.14s ease-out;
}
.tree-editing:hover {
  background: var(--surface-2);
}
.tree-input {
  flex: 1;
  min-width: 0;
  height: 22px;
  margin: -2px 0;
  padding: 0 6px;
  font: inherit;
  color: var(--fg);
  background: var(--bg);
  border: 1px solid var(--focus);
  border-radius: 4px;
  outline: none;
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus) 18%, transparent);
}
.tree-input::placeholder {
  color: var(--muted);
}
.tree-editing.invalid .tree-input {
  border-color: var(--danger);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 18%, transparent);
}
.tree-draft-message {
  margin: 2px 0 4px 40px;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  line-height: 1.4;
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 9%, var(--surface));
}
@keyframes tree-row-in {
  from {
    opacity: 0;
    transform: translateY(-3px);
  }
}
@media (prefers-reduced-motion: reduce) {
  .tree-editing {
    animation: none;
  }
  .tree-more {
    transition: none;
  }
}
.tree-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.tree-folder-icon {
  color: var(--folder);
}
.tree-folder small {
  font-size: 11px;
  color: var(--muted);
}
.tree-chevron {
  transition: transform 0.15s;
  color: var(--muted);
}
.tree-chevron.open {
  transform: rotate(90deg);
}
.tree-file {
  padding-left: 24px;
  color: var(--muted);
}
.tree-file:hover {
  color: var(--fg);
}
.tree-file.active {
  background: var(--primary-soft);
  color: var(--fg);
  font-weight: 600;
}
.tree-folder.selected {
  background: var(--surface-2);
  box-shadow: inset 2px 0 0 var(--primary);
}
.tree-footer {
  padding: 8px 12px;
  border-top: 1px solid var(--border);
  color: var(--muted);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tree-empty {
  padding: 20px 10px;
  color: var(--muted);
  line-height: 1.6;
  margin: 0;
}
</style>
