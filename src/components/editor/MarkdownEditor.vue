<script setup lang="ts">
// Contenedor del editor. Portado de cogni-vault (MarkdownEditor/Index.vue) y
// rehecho como Obsidian/Notion: una sola superficie de escritura, sin modos de
// lectura/escritura ni barras fijas. Las opciones de la nota están en «⋯» y en
// los menús contextuales; el formato, en la barra flotante y el menú «/».
// Otros cambios: aviso de CONFLICT con dos salidas, «Enlazada desde» (backlinks)
// y pestañas accesibles (patrón ARIA tabs).
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { ArrowDownToLine, ArrowUpToLine, BookOpen, Clock, FileText, MoreHorizontal, SlidersHorizontal, TableOfContents, X } from "lucide-vue-next";
import LiveEditor from "./LiveEditor.vue";
import NoteOutline from "./NoteOutline.vue";
import NoteBreadcrumb from "../NoteBreadcrumb.vue";
import PresentationView from "../PresentationView.vue";
import { extractOutline, type OutlineItem } from "../../lib/outline";
import { renderMarkdown } from "../../lib/markdown";
import { sanitizeHtml } from "../../lib/sanitize";
import VersionHistory from "../VersionHistory.vue";
import { api, ApiError, type Note, type NoteSummary } from "../../lib/api";
import { folderPath } from "../../lib/noteTree";
import { useNotesStore } from "../../stores/notes";
import { useContextMenu, type MenuEntry } from "../../composables/useContextMenu";
import type { FileItem, FolderItem, SaveStatus, TabItem } from "../../types";
import "./editor.css";

const props = defineProps<{
  files: FileItem[];
  folders: FolderItem[];
  activeFileId: string | null;
  tabs: TabItem[];
  activeTabId: string | null;
  saveStates: Record<string, SaveStatus>;
  saveErrors: Record<string, unknown>;
  prepareHistory: () => Promise<boolean>;
}>();

const emit = defineEmits<{
  (e: "selectTab", tabId: string): void;
  (e: "closeTab", tabId: string): void;
  (e: "updateContent", fileId: string, content: string): void;
  (e: "updateTitle", fileId: string, title: string): void;
  (e: "saveNow", fileId: string): void;
  (e: "moveNote", fileId: string, folderId: string | null): void;
  (e: "trashNote", fileId: string): void;
  (e: "resolveConflict", fileId: string, action: "reload" | "overwrite"): void;
  (e: "wikiLinkClick", target: string): void;
  (e: "openNote", fileId: string): void;
  (e: "fileRestored", note: Note): void;
  (e: "notify", type: "success" | "error" | "info", text: string): void;
}>();

const store = useNotesStore();
const menu = useContextMenu();
const activeTab = computed(() => props.tabs.find((t) => t.id === props.activeTabId));
const activeFile = computed(() => props.files.find((f) => f.id === (activeTab.value?.fileId ?? props.activeFileId)));
const activeNote = computed(() => (activeFile.value ? store.notes[activeFile.value.id] : undefined));

const folderOptions = computed(() =>
  props.folders.map((f) => ({ id: f.id, label: folderPath(props.folders, f.id).join(" / ") })).sort((a, b) => a.label.localeCompare(b.label, "es")),
);
const breadcrumbs = computed(() => folderPath(props.folders, activeFile.value?.folderId ?? null));

const sourceMode = ref(false);
const widePage = ref(false);
const showProperties = ref(false);
const isHistoryOpen = ref(false);
const isPresenting = ref(false);
const liveEditor = ref<InstanceType<typeof LiveEditor>>();

// ---------- Exportar a PDF ----------
// `window.open()` no funciona en el webview de Tauri (por eso cogni-vault no
// lo traía); `window.print()` sí, en todas las plataformas. Se muestra una
// hoja aparte (ver #print-sheet en el template y @media print en base.css) y
// se oculta el resto de la app mientras se imprime.
const printing = ref(false);
const printHtml = computed(() => sanitizeHtml(renderMarkdown(activeNote.value?.body ?? "")));
async function exportPdf() {
  if (!activeNote.value) return;
  printing.value = true;
  await nextTick();
  const done = () => {
    printing.value = false;
    window.removeEventListener("afterprint", done);
  };
  window.addEventListener("afterprint", done);
  window.print();
}

// ---------- Esquema (títulos de la nota) ----------
const OUTLINE_KEY = "notas.showOutline";
const readOutlinePref = () => {
  try {
    return localStorage.getItem(OUTLINE_KEY) === "1";
  } catch {
    return false;
  }
};
const showOutline = ref(readOutlinePref());
watch(showOutline, (on) => {
  try {
    localStorage.setItem(OUTLINE_KEY, on ? "1" : "0");
  } catch {
    /* Preferencia de comodidad: si no se puede guardar, no pasa nada. */
  }
  if (on) void nextTick(updateActiveHeading);
});
const outline = computed(() => (showOutline.value && activeNote.value ? extractOutline(activeNote.value.body) : []));
const activeHeading = ref(-1);

/** El título activo es el último que ya pasó por la parte de arriba del lienzo. */
function updateActiveHeading() {
  const c = canvas.value;
  const editor = liveEditor.value;
  if (!c || !editor || !outline.value.length) return void (activeHeading.value = -1);
  const limit = c.getBoundingClientRect().top + 64;
  let current = -1;
  for (let i = 0; i < outline.value.length; i++) {
    const top = editor.topOf(outline.value[i].from);
    if (top === null || top > limit) break;
    current = i;
  }
  activeHeading.value = current;
}
watch(outline, () => void nextTick(updateActiveHeading));

function goToHeading(item: OutlineItem) {
  liveEditor.value?.scrollToPos(item.from);
  // En pantallas estrechas el esquema tapa la nota: se cierra al elegir.
  const container = canvas.value?.closest<HTMLElement>(".editor-container");
  if (container && container.clientWidth <= 700) showOutline.value = false;
}

// ---------- Desplazamiento: ir al inicio / al final ----------
const canvas = ref<HTMLElement>();
const scroll = ref({ long: false, atTop: true, atBottom: true });
const EDGE = 40;

function updateScroll() {
  const c = canvas.value;
  if (!c) return;
  const max = c.scrollHeight - c.clientHeight;
  scroll.value = {
    // «Muy grande»: más de pantalla y media de contenido.
    long: c.scrollHeight > c.clientHeight * 1.5,
    atTop: c.scrollTop < EDGE,
    atBottom: max - c.scrollTop < EDGE,
  };
  if (showOutline.value) updateActiveHeading();
}

const smooth = () => (window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth");

function goTop() {
  canvas.value?.scrollTo({ top: 0, behavior: smooth() });
  document.getElementById("note-title")?.focus({ preventScroll: true });
}

function goBottom() {
  // Deja el cursor al final para seguir escribiendo.
  liveEditor.value?.focusEnd();
  // CodeMirror solo pinta lo visible y estima el resto: al llegar abajo la altura
  // real puede crecer, así que se repite hasta tocar fondo.
  let tries = 0;
  const step = () => {
    const c = canvas.value;
    if (!c) return;
    c.scrollTo({ top: c.scrollHeight, behavior: tries ? "auto" : smooth() });
    if (++tries < 4) setTimeout(step, 250);
  };
  step();
}

let resizeObserver: ResizeObserver | undefined;
watch(canvas, (el, old) => {
  old?.removeEventListener("scroll", updateScroll);
  resizeObserver?.disconnect();
  if (!el) return;
  el.addEventListener("scroll", updateScroll, { passive: true });
  resizeObserver = new ResizeObserver(updateScroll);
  resizeObserver.observe(el);
  if (el.firstElementChild) resizeObserver.observe(el.firstElementChild);
  updateScroll();
});
onUnmounted(() => {
  canvas.value?.removeEventListener("scroll", updateScroll);
  resizeObserver?.disconnect();
});

async function openHistory() {
  if (await props.prepareHistory()) isHistoryOpen.value = true;
}

function onShortcut(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || !activeFile.value) return;
  if (e.shiftKey && e.key.toLowerCase() === "o") {
    e.preventDefault();
    showOutline.value = !showOutline.value;
  } else if (!e.shiftKey && e.key.toLowerCase() === "h") {
    e.preventDefault();
    if (isHistoryOpen.value) isHistoryOpen.value = false;
    else void openHistory();
  }
}
onMounted(() => window.addEventListener("keydown", onShortcut));
onUnmounted(() => window.removeEventListener("keydown", onShortcut));

// ---------- Menús ----------
function noteMenu(): MenuEntry[] {
  const file = activeFile.value;
  if (!file) return [];
  return [
    { label: "Guardar ahora", shortcut: "Ctrl+S", action: () => emit("saveNow", file.id) },
    { label: "Historial de versiones…", shortcut: "Ctrl+H", action: openHistory },
    { label: "Propiedades", checked: showProperties.value, action: () => (showProperties.value = !showProperties.value) },
    "separator",
    { label: "Esquema", shortcut: "Ctrl+Mayús+O", checked: showOutline.value, action: () => (showOutline.value = !showOutline.value) },
    { label: "Página ancha", checked: widePage.value, action: () => (widePage.value = !widePage.value) },
    { label: "Ver Markdown sin formato", checked: sourceMode.value, action: () => (sourceMode.value = !sourceMode.value) },
    "separator",
    { label: "Presentación", action: () => (isPresenting.value = true) },
    { label: "Exportar a PDF (imprimir)…", action: exportPdf },
    "separator",
    {
      label: "Mover a",
      children: [
        { label: "Raíz", checked: !file.folderId, action: () => emit("moveNote", file.id, null) },
        ...folderOptions.value.map((f) => ({ label: f.label, checked: file.folderId === f.id, action: () => emit("moveNote", file.id, f.id) })),
      ],
    },
    { label: "Mover a la papelera", danger: true, action: () => emit("trashNote", file.id) },
  ];
}

function openNoteMenu(event: MouseEvent) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  menu.openAt(rect.right - 240, rect.bottom + 4, noteMenu());
}

function tabMenu(event: MouseEvent, tabId: string) {
  const index = props.tabs.findIndex((t) => t.id === tabId);
  const close = (ids: string[]) => ids.forEach((id) => emit("closeTab", id));
  menu.open(event, [
    { label: "Cerrar", shortcut: "Ctrl+W", action: () => close([tabId]) },
    { label: "Cerrar las demás", disabled: props.tabs.length < 2, action: () => close(props.tabs.filter((t) => t.id !== tabId).map((t) => t.id)) },
    { label: "Cerrar las de la derecha", disabled: index >= props.tabs.length - 1, action: () => close(props.tabs.slice(index + 1).map((t) => t.id)) },
    { label: "Cerrar las de la izquierda", disabled: index <= 0, action: () => close(props.tabs.slice(0, index).map((t) => t.id)) },
    "separator",
    { label: "Cerrar todas", action: () => close(props.tabs.map((t) => t.id)) },
  ]);
}

/** Clic en el espacio vacío bajo el texto: escribir al final, como en Notion. */
function onCanvasClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.classList.contains("main-canvas") || target.classList.contains("note-panes") || target.classList.contains("textarea-wrapper"))
    liveEditor.value?.focusEnd();
}

// ---------- Etiquetas ----------
const noteTagList = computed(() => (activeFile.value ? (store.noteTags[activeFile.value.id] ?? []) : []));
const tagInput = ref("");
const tagSuggestions = computed(() => {
  const q = tagInput.value.trim().toLowerCase().replace(/^#/, "");
  if (!q) return [];
  const current = new Set(noteTagList.value.map((t) => t.name));
  return store.tags.filter((t) => !current.has(t.name) && t.name.includes(q)).slice(0, 6);
});
const message = (e: unknown) => (e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e));

async function applyTags(names: string[]) {
  if (!activeFile.value) return;
  try {
    await store.setNoteTags(activeFile.value.id, names);
  } catch (e) {
    emit("notify", "error", `No se pudieron actualizar las etiquetas: ${message(e)}`);
  }
}

function commitTag(raw: string) {
  const name = raw.trim().replace(/^#/, "");
  tagInput.value = "";
  if (!name) return;
  if (noteTagList.value.some((t) => t.name === name)) return;
  void applyTags([...noteTagList.value.map((t) => t.name), name]);
}

function removeTag(name: string) {
  void applyTags(noteTagList.value.map((t) => t.name).filter((n) => n !== name));
}

function onTagKey(e: KeyboardEvent) {
  if (e.key === "Enter" || e.key === ",") {
    e.preventDefault();
    commitTag(tagInput.value);
  } else if (e.key === "Backspace" && !tagInput.value && noteTagList.value.length) {
    removeTag(noteTagList.value[noteTagList.value.length - 1].name);
  }
}

// ---------- Enlazada desde ----------
const backlinks = ref<NoteSummary[]>([]);
let backlinksRequest = 0;
async function loadBacklinks(id: string | undefined) {
  const request = ++backlinksRequest;
  backlinks.value = [];
  if (!id) return;
  try {
    const result = await api.backlinks(id);
    if (request === backlinksRequest) backlinks.value = result;
  } catch {
    /* Los backlinks son informativos: si fallan, la sección queda vacía. */
  }
}
watch(
  () => activeFile.value?.id,
  (id) => {
    isHistoryOpen.value = false;
    void loadBacklinks(id);
    tagInput.value = "";
    if (id) void store.loadNoteTags(id).catch(() => undefined);
    // El editor restaura el desplazamiento de cada nota en el siguiente frame.
    void nextTick(() => requestAnimationFrame(updateScroll));
  },
  { immediate: true },
);
// Tras guardar, los enlaces de otras notas pueden haber cambiado.
watch(
  () => activeFile.value && props.saveStates[activeFile.value.id],
  (state) => state === "saved" && void loadBacklinks(activeFile.value?.id),
);

// ---------- Pestañas con teclado (patrón ARIA de tabs) ----------
function onTabKey(e: KeyboardEvent, index: number) {
  const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
  if (!delta) return;
  e.preventDefault();
  const next = props.tabs[(index + delta + props.tabs.length) % props.tabs.length];
  emit("selectTab", next.id);
  requestAnimationFrame(() => document.getElementById("tab-" + next.id)?.focus());
}

// ---------- Estado de guardado ----------
const fileName = (id: string) => props.files.find((f) => f.id === id)?.name || "Nota eliminada";
const errorOf = (id: string) => props.saveErrors[id];
const isConflict = (id: string) => (errorOf(id) as ApiError | undefined)?.code === "CONFLICT";
const errorText = (id: string) => {
  const e = errorOf(id);
  return e instanceof Error ? e.message : e ? String(e) : "";
};

function saveIndicator(id: string) {
  const status = props.saveStates[id];
  if (status === "dirty") return { cls: "dirty", symbol: "●", label: "cambios sin guardar" };
  if (status === "saving") return { cls: "saving", symbol: "●", label: "guardando" };
  if (status === "error") return { cls: "error", symbol: "⚠", label: "no se pudo guardar" };
  return null;
}

const activeSaveStatus = computed(() => {
  const id = activeFile.value?.id;
  const status = id ? props.saveStates[id] : undefined;
  if (status === "dirty") return { cls: "dirty", text: "Sin guardar" };
  if (status === "saving") return { cls: "saving", text: "Guardando…" };
  if (status === "error") return { cls: "error", text: "⚠ No guardado" };
  return id ? { cls: "saved", text: "Guardado" } : null;
});

const words = (text: string) => (text.trim() ? text.trim().split(/\s+/).length : 0);
</script>

<template>
  <div v-if="tabs.length === 0" class="editor-empty">
    <BookOpen :size="48" class="mde-empty-icon" aria-hidden="true" />
    <p class="mde-empty-title">Ninguna nota abierta</p>
    <p class="mde-empty-subtitle">Crea una nota o elige una del explorador para empezar.</p>
    <p class="mde-empty-hint"><kbd>Ctrl</kbd>+<kbd>N</kbd> nueva nota · <kbd>Ctrl</kbd>+<kbd>P</kbd> abrir nota · <kbd>Ctrl</kbd>+<kbd>K</kbd> buscar</p>
  </div>

  <div v-else class="editor-container" :class="{ 'editor-wide': widePage }">
    <div class="tab-bar">
      <div class="tab-list" role="tablist" aria-label="Notas abiertas">
        <div
          v-for="(tab, index) in tabs"
          :key="tab.id"
          class="tab-item"
          :class="{ 'tab-active': activeTabId === tab.id }"
          @mousedown.middle.prevent="emit('closeTab', tab.id)"
          @contextmenu="tabMenu($event, tab.id)"
        >
          <button
            :id="'tab-' + tab.id"
            type="button"
            role="tab"
            class="tab-select"
            :aria-selected="activeTabId === tab.id"
            aria-controls="note-panel"
            :tabindex="activeTabId === tab.id ? 0 : -1"
            @click="emit('selectTab', tab.id)"
            @keydown="onTabKey($event, index)"
          >
            <FileText :size="12" class="tab-icon" aria-hidden="true" />
            <span class="tab-name">{{ fileName(tab.fileId) }}</span>
            <span v-if="saveIndicator(tab.fileId)" class="tab-save-indicator" :class="saveIndicator(tab.fileId)!.cls">
              <span aria-hidden="true">{{ saveIndicator(tab.fileId)!.symbol }}</span>
              <span class="sr-only">({{ saveIndicator(tab.fileId)!.label }})</span>
            </span>
          </button>
          <button type="button" class="tab-close" :aria-label="`Cerrar «${fileName(tab.fileId)}»`" title="Cerrar (Ctrl+W)" @click="emit('closeTab', tab.id)">
            <X :size="12" aria-hidden="true" />
          </button>
        </div>
      </div>
      <button
        v-if="activeFile"
        type="button"
        class="icon-btn tab-outline"
        :aria-pressed="showOutline"
        aria-label="Esquema de la nota"
        title="Mostrar u ocultar el esquema (Ctrl+Mayús+O)"
        @click="showOutline = !showOutline"
      >
        <TableOfContents :size="16" aria-hidden="true" />
      </button>
      <button
        v-if="activeFile"
        type="button"
        class="icon-btn tab-more"
        aria-haspopup="menu"
        aria-label="Opciones de la nota"
        title="Opciones de la nota"
        @click="openNoteMenu"
      >
        <MoreHorizontal :size="18" aria-hidden="true" />
      </button>
    </div>

    <div
      v-if="activeFile && activeNote"
      id="note-panel"
      class="editor-content-wrapper"
      role="tabpanel"
      :aria-labelledby="activeTab ? 'tab-' + activeTab.id : undefined"
    >
      <div class="editor-body">
        <div class="canvas-area">
          <div ref="canvas" class="main-canvas" @click="onCanvasClick">
            <div class="textarea-wrapper">
              <div class="editor-metadata-bar">
                <NoteBreadcrumb class="editor-breadcrumbs" :segments="breadcrumbs" :name="activeNote.title" />
                <label class="sr-only" for="note-title">Título de la nota</label>
                <textarea
                  id="note-title"
                  rows="1"
                  class="metadata-title-input"
                  placeholder="Sin título"
                  :value="activeNote.title"
                  @input="emit('updateTitle', activeFile.id, ($event.target as HTMLTextAreaElement).value)"
                  @keydown.enter.prevent="liveEditor?.focus()"
                  @keydown.down.prevent="liveEditor?.focus()"
                ></textarea>
                <button
                  v-if="showProperties"
                  type="button"
                  class="note-properties-toggle"
                  aria-expanded="true"
                  aria-controls="note-properties"
                  @click="showProperties = false"
                >
                  <SlidersHorizontal :size="13" aria-hidden="true" /> Ocultar propiedades
                </button>
                <div v-if="showProperties" id="note-properties" class="metadata-selectors">
                  <label>
                    Carpeta
                    <select
                      class="metadata-select"
                      :value="activeFile.folderId ?? ''"
                      @change="emit('moveNote', activeFile.id, ($event.target as HTMLSelectElement).value || null)"
                    >
                      <option value="">Raíz</option>
                      <option v-for="folder in folderOptions" :key="folder.id" :value="folder.id">{{ folder.label }}</option>
                    </select>
                  </label>
                  <label class="tag-field" for="note-tag-input">
                    Etiquetas
                    <div class="tag-editor">
                      <ul class="tag-chip-list">
                        <li v-for="tag in noteTagList" :key="tag.id" class="tag-chip">
                          <span>#{{ tag.name }}</span>
                          <button type="button" class="tag-chip-remove" :aria-label="`Quitar etiqueta ${tag.name}`" @click="removeTag(tag.name)">×</button>
                        </li>
                      </ul>
                      <input
                        id="note-tag-input"
                        v-model="tagInput"
                        type="text"
                        class="tag-input"
                        placeholder="Añadir etiqueta…"
                        list="note-tag-suggestions"
                        @keydown="onTagKey"
                        @blur="tagInput.trim() && commitTag(tagInput)"
                      />
                      <datalist id="note-tag-suggestions">
                        <option v-for="t in tagSuggestions" :key="t.id" :value="t.name" />
                      </datalist>
                    </div>
                  </label>
                  <p class="metadata-dates">Creada el {{ new Date(activeNote.createdAt).toLocaleString("es") }} · versión {{ activeNote.version }}</p>
                </div>
              </div>

              <div v-if="saveStates[activeFile.id] === 'error'" class="note-save-error" role="alert">
                <template v-if="isConflict(activeFile.id)">
                  <p>Esta nota cambió fuera del editor (por ejemplo, al restaurar una versión). Tus cambios siguen aquí.</p>
                  <div class="note-save-actions">
                    <button type="button" class="btn" @click="emit('resolveConflict', activeFile.id, 'reload')">Recargar la guardada (descarta lo mío)</button>
                    <button type="button" class="btn primary" @click="emit('resolveConflict', activeFile.id, 'overwrite')">Sobrescribir con lo mío</button>
                  </div>
                </template>
                <template v-else>
                  <p>No se pudo guardar: {{ errorText(activeFile.id) }}. Tus cambios siguen aquí.</p>
                  <div class="note-save-actions">
                    <button type="button" class="btn" @click="emit('saveNow', activeFile.id)">Reintentar</button>
                  </div>
                </template>
              </div>

              <div class="note-panes">
                <LiveEditor
                  ref="liveEditor"
                  :file-id="activeFile.id"
                  :content="activeNote.body"
                  :source-mode="sourceMode"
                  :files="files"
                  :open-file-ids="tabs.map((tab) => tab.fileId)"
                  @change="(id, content) => emit('updateContent', id, content)"
                  @wiki-link-click="(target) => emit('wikiLinkClick', target)"
                  @notify="(type, text) => emit('notify', type, text)"
                />
              </div>

              <section v-if="backlinks.length" class="backlinks" aria-labelledby="backlinks-title">
                <h2 id="backlinks-title">Enlazada desde</h2>
                <ul>
                  <li v-for="link in backlinks" :key="link.id">
                    <button type="button" class="backlink" @click="emit('openNote', link.id)">{{ link.title }}</button>
                  </li>
                </ul>
              </section>
            </div>
          </div>

          <div v-if="scroll.long" class="jump-buttons" role="group" aria-label="Desplazamiento de la nota">
            <button v-show="!scroll.atTop" type="button" class="jump-btn" aria-label="Ir al inicio de la nota" title="Ir al inicio" @click="goTop">
              <ArrowUpToLine :size="16" aria-hidden="true" />
            </button>
            <button v-show="!scroll.atBottom" type="button" class="jump-btn" aria-label="Ir al final de la nota" title="Ir al final" @click="goBottom">
              <ArrowDownToLine :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>

        <NoteOutline
          v-if="showOutline"
          :items="outline"
          :active-index="activeHeading"
          :note-id="activeFile.id"
          @select="goToHeading"
          @close="showOutline = false"
        />
      </div>

      <footer class="status-bar">
        <div class="status-left">
          <span>{{ words(activeNote.body) }} palabras</span>
          <span>{{ activeNote.body.length }} caracteres</span>
          <span v-if="sourceMode" class="status-mode">Markdown sin formato</span>
        </div>
        <div class="status-right">
          <span v-if="activeSaveStatus" class="status-save" :class="activeSaveStatus.cls">{{ activeSaveStatus.text }}</span>
          <Clock :size="11" class="clock-icon" aria-hidden="true" />
          <span>Modificada {{ new Date(activeNote.updatedAt).toLocaleTimeString("es", { hour: "numeric", minute: "2-digit" }) }}</span>
        </div>
      </footer>
    </div>
    <div v-else-if="activeFile" class="editor-closed" role="status">Abriendo nota…</div>
    <div v-else class="editor-closed">Se cerró la nota activa.</div>

    <VersionHistory
      v-if="isHistoryOpen && activeFile"
      :note-id="activeFile.id"
      @close="isHistoryOpen = false"
      @restored="
        (note) => {
          isHistoryOpen = false;
          emit('fileRestored', note);
        }
      "
    />
    <PresentationView v-if="isPresenting && activeNote" :title="activeNote.title" :source="activeNote.body" @close="isPresenting = false" />
  </div>
  <Teleport to="body">
    <div v-if="printing" id="print-sheet" class="markdown-reading">
      <h1>{{ activeNote?.title }}</h1>
      <div v-html="printHtml"></div>
    </div>
  </Teleport>
</template>
