<script setup lang="ts">
// Shell de la app. Portado de cogni-vault (views/NotesView/Index.vue) sin
// cerebros, facetas, copiloto ni inspector epistémico. Coordina pestañas,
// autoguardado con control de versión, atajos y diálogos.
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Archive, PanelLeft, Palette, Search, Settings, Tag, Trash2 } from "lucide-vue-next";
import Sidebar from "./components/Sidebar.vue";
import MarkdownEditor from "./components/editor/MarkdownEditor.vue";
import CommandPalette, { type PaletteAction } from "./components/CommandPalette.vue";
import SearchPanel from "./components/SearchPanel.vue";
import TrashDialog from "./components/TrashDialog.vue";
import VaultDialog from "./components/VaultDialog.vue";
import AppearanceDialog from "./components/AppearanceDialog.vue";
import TagsDialog from "./components/TagsDialog.vue";
import PreferencesDialog from "./components/PreferencesDialog.vue";
import Modal from "./components/Modal.vue";
import ContextMenu from "./components/ContextMenu.vue";
import { api, ApiError, type Note } from "./lib/api";
import { parseFrontmatter } from "./lib/frontmatter";
import { useNotesStore } from "./stores/notes";
import { useSaveCoordinator } from "./composables/useSaveCoordinator";
import { useToast } from "./composables/useToast";
import { useConfirm } from "./composables/useConfirm";
import { useContextMenu } from "./composables/useContextMenu";
import { useAppearance } from "./composables/useAppearance";
import { usePreferences } from "./composables/usePreferences";
import type { TabItem } from "./types";

const store = useNotesStore();
const toast = useToast();
const { pending: confirmRequest, confirm, answer } = useConfirm();
const contextMenu = useContextMenu();
const preferences = usePreferences();
const message = (e: unknown) => (e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e));

const tabs = ref<TabItem[]>([]);
const activeTabId = ref<string | null>(null);
const activeFileId = ref<string | null>(null);
const showSidebar = ref(window.matchMedia("(min-width: 760px)").matches);
const creatingNote = ref(false);
const dialog = ref<"" | "palette" | "search" | "trash" | "vault" | "appearance" | "tags" | "preferences">("");
const sidebar = ref<InstanceType<typeof Sidebar>>();

const titleOf = (id: string) => store.notes[id]?.title || store.summaries.find((s) => s.id === id)?.title || "Sin título";

// ---------- Autoguardado ----------
const { saveStates, saveErrors, hasUnsavedChanges, scheduleSave, persist, saveNow, flushUnsaved, clearSaveTracking } = useSaveCoordinator({
  exists: (id) => !!store.notes[id],
  save: (id) => store.saveNote(id),
  delay: () => preferences.state.autosaveDelayMs,
  // Reintentar no arregla un conflicto de versión ni un dato inválido.
  isRetryable: (e) => !(e instanceof ApiError && ["CONFLICT", "VALIDATION", "NOT_FOUND"].includes(e.code)),
  onSaveSuccess: (id) => toast.success(`«${titleOf(id)}» guardada`),
  onSaveError: (id, e) => {
    if (e instanceof ApiError && e.code === "CONFLICT") toast.warning(`«${titleOf(id)}» cambió fuera del editor; elige qué versión conservar`);
    else toast.error(`No se pudo guardar «${titleOf(id)}»: ${message(e)}`);
  },
});

function updateContent(id: string, content: string) {
  const note = store.notes[id];
  if (!note || note.body === content) return;
  note.body = content;
  scheduleSave(id);
}

function updateTitle(id: string, title: string) {
  const note = store.notes[id];
  if (!note) return;
  note.title = title;
  scheduleSave(id);
}

async function resolveConflict(id: string, action: "reload" | "overwrite") {
  try {
    if (action === "reload") {
      clearSaveTracking(id);
      await store.reloadNote(id);
      toast.info(`Se recargó «${titleOf(id)}» tal como está guardada`);
    } else {
      await store.adoptServerVersion(id);
      await saveNow(id);
    }
  } catch (e) {
    toast.error(message(e));
  }
}

// ---------- Pestañas ----------
let openRequest = 0;
async function openFile(id: string) {
  const request = ++openRequest;
  try {
    await store.ensureNote(id);
  } catch (e) {
    toast.error(`No se pudo abrir la nota: ${message(e)}`);
    return;
  }
  if (request !== openRequest) return;
  activeFileId.value = id;
  const existing = tabs.value.find((t) => t.fileId === id);
  if (existing) activeTabId.value = existing.id;
  else {
    const tab = { id: `tab-${Date.now()}-${id}`, fileId: id };
    tabs.value.push(tab);
    activeTabId.value = tab.id;
  }
  if (window.matchMedia("(max-width: 759px)").matches) showSidebar.value = false;
}

function selectTab(tabId: string) {
  const tab = tabs.value.find((t) => t.id === tabId);
  if (!tab) return;
  activeTabId.value = tabId;
  activeFileId.value = tab.fileId;
}

function dropTab(tabId: string) {
  const index = tabs.value.findIndex((t) => t.id === tabId);
  if (index < 0) return;
  tabs.value.splice(index, 1);
  if (activeTabId.value === tabId) {
    const next = tabs.value[Math.min(index, tabs.value.length - 1)];
    activeTabId.value = next?.id ?? null;
    activeFileId.value = next?.fileId ?? null;
  }
}

async function closeTab(tabId: string) {
  const tab = tabs.value.find((t) => t.id === tabId);
  if (!tab) return;
  if (saveStates.value[tab.fileId] && saveStates.value[tab.fileId] !== "saved" && !(await persist(tab.fileId))) {
    const discard = await confirm({
      title: "Cambios sin guardar",
      message: `«${titleOf(tab.fileId)}» no se pudo guardar. Si cierras la pestaña, perderás esos cambios.`,
      confirmLabel: "Cerrar y descartar",
      destructive: true,
    });
    if (!discard) return;
    await store.reloadNote(tab.fileId).catch(() => undefined);
  }
  clearSaveTracking(tab.fileId);
  dropTab(tabId);
}

/** Cierra las pestañas de notas que ya no existen (papelera, purga, importación). */
function pruneTabs() {
  for (const tab of [...tabs.value]) {
    if (!store.summaries.some((s) => s.id === tab.fileId)) {
      clearSaveTracking(tab.fileId);
      dropTab(tab.id);
    }
  }
}

async function reloadAll() {
  try {
    await store.loadAll();
  } catch (e) {
    toast.error(`No se pudieron cargar las notas: ${message(e)}`);
  }
  pruneTabs();
}

// ---------- Notas ----------
async function createFile(folderId: string | null = null, title?: string, body = "") {
  if (creatingNote.value) return;
  creatingNote.value = true;
  try {
    const note = await store.createNote({ title: title || `Nota sin título ${store.summaries.length + 1}`, body, folderId });
    await openFile(note.id);
    if (!title) {
      await nextTick();
      const input = document.getElementById("note-title") as HTMLTextAreaElement | null;
      input?.focus();
      input?.select();
    }
    return note.id;
  } catch (e) {
    toast.error(`No se pudo crear la nota: ${message(e)}`);
  } finally {
    creatingNote.value = false;
  }
}

async function createDailyNote() {
  const today = new Date();
  const title = today.toISOString().slice(0, 10);
  const existing = store.summaries.find((s) => s.title === title);
  if (existing) return openFile(existing.id);
  const readable = today.toLocaleDateString("es", { weekday: "long", year: "numeric", month: "long", day: "numeric" });
  await createFile(null, title, `> ${readable}\n\n## Registro del día\n\n- [ ] `);
}

async function trashNote(id: string) {
  await persist(id);
  try {
    await store.trashNote(id);
    clearSaveTracking(id);
    for (const tab of tabs.value.filter((t) => t.fileId === id)) dropTab(tab.id);
    toast.success("Nota movida a la papelera");
  } catch (e) {
    toast.error(message(e));
  }
}

async function moveNote(id: string, folderId: string | null) {
  // Primero guardar: mover incrementa la versión y un guardado en vuelo daría CONFLICT.
  if (!(await persist(id))) return toast.error("Guarda la nota antes de moverla");
  try {
    await store.moveNote(id, folderId);
  } catch (e) {
    toast.error(message(e));
  }
}

function fileRestored(note: Note) {
  clearSaveTracking(note.id);
  toast.success(`Versión restaurada en «${note.title}»`);
}

async function wikiLinkClick(target: string) {
  try {
    const match = await api.resolveLink(target);
    if (match) return openFile(match.id);
    // Como en Obsidian: seguir un enlace a una nota inexistente la crea.
    const id = await createFile(null, target.trim());
    if (id) toast.success(`Nota «${target.trim()}» creada`);
  } catch (e) {
    toast.error(message(e));
  }
}

async function importMarkdownFiles(folderId: string | null, files: File[]) {
  let ok = 0;
  const failed: string[] = [];
  for (const file of files) {
    try {
      const { data, body } = parseFrontmatter(await file.text());
      await store.createNote({ title: data.title || file.name.replace(/\.(md|markdown)$/i, ""), body, folderId });
      ok++;
    } catch (e) {
      failed.push(`${file.name} (${message(e)})`);
    }
  }
  if (ok) toast.success(`${ok} nota(s) importada(s)`);
  if (failed.length) toast.error(`No se importaron: ${failed.join(", ")}`);
}

// ---------- Carpetas ----------
async function createFolder(parentId: string | null, name: string) {
  try {
    await store.createFolder(name, parentId);
  } catch (e) {
    toast.error(message(e));
  }
}

async function renameFolder(id: string, name: string) {
  try {
    await store.renameFolder(id, name);
  } catch (e) {
    toast.error(message(e));
  }
}

async function moveFolder(id: string, target: string | null) {
  try {
    await store.moveFolder(id, target);
  } catch (e) {
    toast.error(message(e));
    if (e instanceof ApiError && e.code === "CONFLICT") await store.refreshFolders();
  }
}

async function deleteFolder(id: string) {
  if (!(await flushUnsaved())) return toast.error("Hay notas sin guardar; guárdalas antes de mover la carpeta a la papelera");
  try {
    const r = await store.trashFolder(id);
    pruneTabs();
    toast.success(`Movidas a la papelera ${r.folders} carpeta(s) y ${r.notes} nota(s)`);
  } catch (e) {
    toast.error(message(e));
  }
}

// ---------- Paleta y atajos ----------
const paletteActions: PaletteAction[] = [
  { id: "new", label: "Nueva nota", hint: "Ctrl+N" },
  { id: "daily", label: "Nota diaria de hoy" },
  { id: "search", label: "Buscar en el contenido", hint: "Ctrl+K" },
  { id: "trash", label: "Abrir la papelera" },
  { id: "tags", label: "Etiquetas: ver y gestionar" },
  { id: "preferences", label: "Preferencias" },
  { id: "vault", label: "Copia de seguridad: exportar, importar, migrar" },
  { id: "sidebar", label: "Mostrar u ocultar el explorador", hint: "Ctrl+\\" },
  { id: "appearance", label: "Apariencia: temas y fragmentos CSS" },
];

function runAction(id: string) {
  if (id === "new") void createFile();
  else if (id === "daily") void createDailyNote();
  else if (id === "search") dialog.value = "search";
  else if (id === "trash") dialog.value = "trash";
  else if (id === "tags") dialog.value = "tags";
  else if (id === "preferences") dialog.value = "preferences";
  else if (id === "vault") dialog.value = "vault";
  else if (id === "appearance") dialog.value = "appearance";
  else if (id === "sidebar") showSidebar.value = !showSidebar.value;
}

function onKey(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
  const key = e.key.toLowerCase();
  const toggle = (name: typeof dialog.value) => (dialog.value = dialog.value === name ? "" : name);
  if (key === "p") toggle("palette");
  else if (key === "k") toggle("search");
  else if (key === "n" && !e.shiftKey) void createFile();
  else if (key === "s") activeFileId.value && void saveNow(activeFileId.value);
  else if (key === "w") activeTabId.value && void closeTab(activeTabId.value);
  else if (key === "\\") showSidebar.value = !showSidebar.value;
  else if (key === "f" && e.shiftKey) {
    showSidebar.value = true;
    void nextTick(() => sidebar.value?.focusFilter());
  } else return;
  e.preventDefault();
}

// ---------- Menú contextual global ----------
// El menú del navegador (Recargar, Inspeccionar…) no pinta nada en una app de
// escritorio. Cada zona abre el suyo; los campos de texto, uno de portapapeles.
// En desarrollo, Mayús + clic derecho sigue dando el menú nativo para depurar.
function onContextMenu(e: MouseEvent) {
  if (e.defaultPrevented) return;
  if (import.meta.env.DEV && e.shiftKey) return;
  e.preventDefault();
  const field = (e.target as HTMLElement).closest<HTMLInputElement | HTMLTextAreaElement>(
    "input:not([type=checkbox]):not([type=radio]):not([type=file]), textarea",
  );
  if (!field || field.disabled) return;
  const start = field.selectionStart ?? 0;
  const end = field.selectionEnd ?? 0;
  const selected = field.value.slice(start, end);
  const replace = (text: string) => {
    field.focus();
    field.setRangeText(text, start, end, "end");
    field.dispatchEvent(new Event("input", { bubbles: true }));
  };
  contextMenu.open(e, [
    {
      label: "Cortar",
      shortcut: "Ctrl+X",
      disabled: !selected || field.readOnly,
      action: async () => (await navigator.clipboard.writeText(selected), replace("")),
    },
    { label: "Copiar", shortcut: "Ctrl+C", disabled: !selected, action: () => navigator.clipboard.writeText(selected) },
    {
      label: "Pegar",
      shortcut: "Ctrl+V",
      disabled: field.readOnly,
      action: async () => {
        try {
          replace(await navigator.clipboard.readText());
        } catch {
          toast.info("El sistema no deja pegar desde el menú; usa Ctrl+V");
        }
      },
    },
    "separator",
    { label: "Seleccionar todo", shortcut: "Ctrl+A", action: () => (field.focus(), field.select()) },
  ]);
}

// ---------- Ciclo de vida ----------
let unlistenClose: (() => void) | undefined;
onMounted(async () => {
  window.addEventListener("keydown", onKey);
  document.addEventListener("contextmenu", onContextMenu);
  // El tema no debe impedir abrir las notas: si falla, se queda el predeterminado.
  void useAppearance()
    .reload()
    .catch((e) => toast.warning(`No se pudo aplicar el tema: ${message(e)}`));
  await reloadAll();
  // Al cerrar la ventana: guardar lo pendiente y, si no se puede, preguntar.
  const win = getCurrentWindow();
  unlistenClose = await win.onCloseRequested(async (event) => {
    if (!hasUnsavedChanges.value) return;
    event.preventDefault();
    if (
      (await flushUnsaved()) ||
      (await confirm({
        title: "Cambios sin guardar",
        message: "Algunas notas no se pudieron guardar. Si cierras ahora, perderás esos cambios.",
        confirmLabel: "Cerrar sin guardar",
        destructive: true,
      }))
    )
      await win.destroy();
  });
});
onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  document.removeEventListener("contextmenu", onContextMenu);
  unlistenClose?.();
});

const prepareHistory = async () => (activeFileId.value ? persist(activeFileId.value) : true);
const noteCount = computed(() => store.summaries.length);
</script>

<template>
  <div class="app-shell">
    <header class="app-header">
      <button
        type="button"
        class="icon-btn"
        :aria-pressed="showSidebar"
        aria-label="Mostrar explorador"
        title="Mostrar u ocultar el explorador (Ctrl+\)"
        @click="showSidebar = !showSidebar"
      >
        <PanelLeft :size="16" aria-hidden="true" />
      </button>
      <span class="app-brand">
        <svg class="app-logo" viewBox="0 0 512 512" width="20" height="20" aria-hidden="true">
          <rect width="512" height="512" rx="114" fill="#1C1B1A" />
          <path d="M150,362 Q210,310 262,258 T372,156" fill="none" stroke="#E2754A" stroke-opacity="0.5" stroke-width="16" stroke-linecap="round" />
          <circle cx="372" cy="156" r="30" fill="#E2754A" fill-opacity="0.75" />
          <circle cx="262" cy="258" r="38" fill="#E2754A" />
          <circle cx="150" cy="362" r="46" fill="#F7F5F2" />
        </svg>
        <span class="app-title">Strand</span>
      </span>
      <span class="app-count">{{ noteCount }} notas</span>
      <div class="app-actions">
        <button type="button" class="header-btn" title="Abrir nota o acción (Ctrl+P)" @click="dialog = 'palette'"><span>Ir a…</span><kbd>Ctrl P</kbd></button>
        <button type="button" class="header-btn" title="Buscar en el contenido (Ctrl+K)" @click="dialog = 'search'">
          <Search :size="14" aria-hidden="true" /><span>Buscar</span><kbd>Ctrl K</kbd>
        </button>
        <button type="button" class="icon-btn" aria-label="Papelera" title="Papelera" @click="dialog = 'trash'">
          <Trash2 :size="16" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" aria-label="Etiquetas" title="Etiquetas" @click="dialog = 'tags'">
          <Tag :size="16" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" aria-label="Apariencia" title="Apariencia: temas y fragmentos CSS" @click="dialog = 'appearance'">
          <Palette :size="16" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" aria-label="Preferencias" title="Preferencias" @click="dialog = 'preferences'">
          <Settings :size="16" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" aria-label="Copia de seguridad" title="Copia de seguridad" @click="dialog = 'vault'">
          <Archive :size="16" aria-hidden="true" />
        </button>
      </div>
    </header>

    <div v-if="store.loadError" class="load-error" role="alert">
      No se pudieron cargar las notas: {{ store.loadError }}
      <button type="button" class="btn" @click="reloadAll">Reintentar</button>
    </div>

    <div class="app-main">
      <Sidebar
        v-show="showSidebar"
        ref="sidebar"
        :files="store.files"
        :folders="store.folderItems"
        :active-file-id="activeFileId"
        :loading="store.loading && !store.summaries.length"
        :error="store.loadError"
        :creating="creatingNote"
        @select-file="openFile"
        @create-file="(folderId) => createFile(folderId)"
        @create-folder="createFolder"
        @rename-folder="renameFolder"
        @move-folder="moveFolder"
        @delete-folder="deleteFolder"
        @import-markdown-files="importMarkdownFiles"
        @move-note="moveNote"
        @trash-note="trashNote"
      />
      <main class="app-content">
        <MarkdownEditor
          :files="store.files"
          :folders="store.folderItems"
          :active-file-id="activeFileId"
          :tabs="tabs"
          :active-tab-id="activeTabId"
          :save-states="saveStates"
          :save-errors="saveErrors"
          :prepare-history="prepareHistory"
          @select-tab="selectTab"
          @close-tab="closeTab"
          @update-content="updateContent"
          @update-title="updateTitle"
          @save-now="(id) => saveNow(id)"
          @move-note="moveNote"
          @trash-note="trashNote"
          @resolve-conflict="resolveConflict"
          @wiki-link-click="wikiLinkClick"
          @open-note="openFile"
          @file-restored="fileRestored"
          @notify="(type, text) => toast.show(text, type)"
        />
      </main>
    </div>

    <CommandPalette
      v-if="dialog === 'palette'"
      :files="store.files"
      :actions="paletteActions"
      @close="dialog = ''"
      @select-file="openFile"
      @action="runAction"
      @create-file="(title) => createFile(null, title)"
    />
    <SearchPanel v-if="dialog === 'search'" :folders="store.folderItems" @close="dialog = ''" @select-file="openFile" />
    <TrashDialog v-if="dialog === 'trash'" @close="dialog = ''" @changed="reloadAll" />
    <TagsDialog v-if="dialog === 'tags'" @close="dialog = ''" @select-file="openFile" />
    <PreferencesDialog v-if="dialog === 'preferences'" @close="dialog = ''" />
    <VaultDialog v-if="dialog === 'vault'" @close="dialog = ''" @changed="reloadAll" />
    <AppearanceDialog v-if="dialog === 'appearance'" @close="dialog = ''" />

    <Modal v-if="confirmRequest" :title="confirmRequest.title" size="sm" @close="answer(false)">
      <p class="confirm-message">{{ confirmRequest.message }}</p>
      <template #footer>
        <button type="button" class="btn" @click="answer(false)">Cancelar</button>
        <button type="button" class="btn" :class="confirmRequest.destructive ? 'danger solid' : 'primary'" @click="answer(true)">
          {{ confirmRequest.confirmLabel || "Aceptar" }}
        </button>
      </template>
    </Modal>

    <ContextMenu />

    <ul class="toasts" aria-live="polite" aria-label="Avisos">
      <li v-for="t in toast.toasts.value" :key="t.id" class="toast" :class="t.type" :role="t.type === 'error' ? 'alert' : undefined">
        <span>{{ t.text }}</span>
        <button type="button" class="toast-close" aria-label="Cerrar aviso" @click="toast.dismiss(t.id)">×</button>
      </li>
    </ul>
  </div>
</template>

<style>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.app-header {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 44px;
  padding: 4px 10px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}
.app-brand {
  display: flex;
  align-items: center;
  gap: 6px;
}
.app-logo {
  flex-shrink: 0;
  border-radius: 4px;
}
.app-title {
  font-weight: 700;
  letter-spacing: -0.01em;
}
.app-count {
  color: var(--muted);
  font-size: 12px;
}
.app-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 4px;
}
.header-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 30px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--muted);
  font-size: 12px;
}
.header-btn:hover {
  color: var(--fg);
}
.load-error {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 14px;
  background: color-mix(in srgb, var(--danger) 10%, var(--bg));
  border-bottom: 1px solid var(--danger);
}
.app-main {
  flex: 1;
  display: flex;
  min-height: 0;
}
.app-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.confirm-message {
  margin: 0;
  line-height: 1.6;
}
.toasts {
  position: fixed;
  right: 16px;
  bottom: 16px;
  z-index: 9000;
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: min(420px, calc(100vw - 32px));
}
.toast {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-left: 4px solid var(--primary);
  border-radius: 8px;
  background: var(--bg);
  box-shadow: var(--shadow);
  font-size: 13px;
}
.toast span {
  flex: 1;
}
.toast.success {
  border-left-color: var(--success);
}
.toast.warning {
  border-left-color: var(--warning);
}
.toast.error {
  border-left-color: var(--danger);
}
.toast-close {
  color: var(--muted);
  font-size: 16px;
  line-height: 1;
  padding: 0 2px;
}
@media (max-width: 759px) {
  .app-count,
  .header-btn kbd {
    display: none;
  }
  .app-main .file-explorer {
    position: absolute;
    top: 44px;
    bottom: 0;
    left: 0;
    z-index: 100;
    box-shadow: var(--shadow);
  }
}
</style>
