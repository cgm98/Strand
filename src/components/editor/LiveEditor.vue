<script setup lang="ts">
// Editor único, sin modos de lectura/escritura (como Obsidian y Notion). Base
// portada de cogni-vault; las opciones aparecen donde se usan:
// - barra flotante al seleccionar texto;
// - botón «+» en el margen de cada línea y menú «/» con todos los bloques;
// - menú contextual (clic derecho) con formato, párrafo e insertar;
// - los enlaces renderizados se siguen con un clic (Ctrl+clic siempre);
// - pegar o arrastrar imágenes las guarda como adjunto.
import { onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { EditorState, Transaction, type TransactionSpec } from "@codemirror/state";
import { EditorView, keymap, drawSelection, placeholder, highlightActiveLine } from "@codemirror/view";
import { history, historyKeymap, defaultKeymap, indentWithTab, undo, redo } from "@codemirror/commands";
import { markdown, markdownKeymap } from "@codemirror/lang-markdown";
import { syntaxHighlighting, HighlightStyle, syntaxTree } from "@codemirror/language";
import type { SyntaxNode } from "@lezer/common";
import { tags } from "@lezer/highlight";
import { autocompletion, completionKeymap, type Completion, type CompletionContext } from "@codemirror/autocomplete";
import { GFM } from "@lezer/markdown";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Bold, Code, Italic, Link, Link2, Plus, Strikethrough } from "lucide-vue-next";
import { isImageFile, pickImageFiles, serializeImage, uploadImages } from "../../lib/images";
import { liveDecorations, liveModeEffect, focusEffect } from "./liveDecorations";
import { previewBlocks, selectionTouches } from "./liveDocument";
import {
  copySelection,
  currentBlock,
  insertLink,
  insertTemplate,
  insertWikiLink,
  openInsertMenuAt,
  pasteText,
  setBlock,
  toggleWrap,
  type BlockKind,
} from "./editorCommands";
import { useContextMenu, type MenuEntry } from "../../composables/useContextMenu";
import type { FileItem } from "../../types";
import "katex/dist/katex.min.css";
import "./document.css";
import "./live-editor.css";

// `defaultHighlightStyle` trae colores fijos (p. ej. `tags.url` en azul #219)
// que no conocen los temas de la app y pisaban el `color: var(--link)` de
// `.cm-live-link` en los enlaces visibles (el texto en sí, no el subrayado:
// el subrayado hereda el color del ancestro, pero el propio span de `tags.url`
// se pinta con su color directo, que gana sobre el heredado). Resto de reglas
// igual que el estilo por defecto; solo cambia el color de enlace/URL.
const markdownHighlightStyle = HighlightStyle.define([
  { tag: tags.heading, textDecoration: "underline", fontWeight: "bold" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: [tags.url, tags.link], color: "var(--link)" },
]);

const props = defineProps<{
  fileId: string;
  content: string;
  sourceMode?: boolean;
  files: FileItem[];
  openFileIds: string[];
}>();
const emit = defineEmits<{
  (e: "change", id: string, content: string): void;
  (e: "wikiLinkClick", target: string): void;
  (e: "notify", type: "success" | "error" | "info", text: string): void;
}>();

const menu = useContextMenu();
const wrapper = ref<HTMLElement>();
const host = ref<HTMLElement>();
const uploading = ref(false);
const bubble = reactive({ visible: false, x: 0, y: 0 });
const plus = reactive({ visible: false, top: 0, line: 1 });
const sessions = new Map<string, { state: EditorState; scroll: number }>();
let view: EditorView | undefined;
let currentId = "";
let restoring = false;
let frame = 0;
const external = Transaction.addToHistory.of(false);

// ---------- Menú «/» ----------

interface SlashCommand {
  label: string;
  detail: string;
  keywords: string;
  run: (view: EditorView) => void;
}
const block = (kind: BlockKind) => (v: EditorView) => setBlock(v, kind);
const slashCommands: SlashCommand[] = [
  { label: "Texto", detail: "Párrafo normal", keywords: "texto parrafo p", run: block("paragraph") },
  { label: "Título 1", detail: "Encabezado grande", keywords: "titulo h1 encabezado", run: block("h1") },
  { label: "Título 2", detail: "Encabezado mediano", keywords: "titulo h2 encabezado", run: block("h2") },
  { label: "Título 3", detail: "Encabezado pequeño", keywords: "titulo h3 encabezado", run: block("h3") },
  { label: "Lista", detail: "Lista con viñetas", keywords: "lista vinetas ul bullet", run: block("bullet") },
  { label: "Lista numerada", detail: "1, 2, 3…", keywords: "lista numerada ol ordenada", run: block("ordered") },
  { label: "Tarea", detail: "Casilla para marcar", keywords: "tarea casilla todo checkbox", run: block("task") },
  { label: "Cita", detail: "Bloque de cita", keywords: "cita quote", run: block("quote") },
  { label: "Imagen", detail: "Desde un archivo", keywords: "imagen foto img", run: () => void pickImages() },
  { label: "Tabla", detail: "Tabla editable", keywords: "tabla table", run: (v) => insertTemplate(v, "table") },
  { label: "Código", detail: "Bloque de código", keywords: "codigo code", run: (v) => insertTemplate(v, "code") },
  { label: "Fórmula", detail: "Matemáticas con LaTeX", keywords: "formula matematica latex katex", run: (v) => insertTemplate(v, "math") },
  { label: "Diagrama", detail: "Mermaid", keywords: "diagrama mermaid grafico flujo", run: (v) => insertTemplate(v, "mermaid") },
  { label: "Separador", detail: "Línea horizontal", keywords: "separador linea hr divisor", run: (v) => insertTemplate(v, "rule") },
  { label: "Enlace a nota", detail: "[[Otra nota]]", keywords: "enlace nota wikilink", run: insertWikiLink },
];
const normalize = (s: string) => s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

function completions(context: CompletionContext) {
  const wiki = context.matchBefore(/\[\[[^\]\n]*/);
  if (wiki)
    return {
      from: wiki.from + 2,
      options: props.files
        .filter((f) => f.id !== props.fileId)
        .map((f): Completion => ({
          label: f.name,
          type: "text",
          apply: (editor, _c, from, to) => {
            const suffix = editor.state.sliceDoc(to, to + 2) === "]]" ? "" : "]]";
            editor.dispatch({ changes: { from, to, insert: f.name + suffix }, selection: { anchor: from + f.name.length + suffix.length } });
          },
        })),
    };
  // «/» solo al principio de una línea (o tras espacios), como en Notion.
  const slash = context.matchBefore(/\/[\p{L}\d ]*$/u);
  if (!slash || context.state.sliceDoc(context.state.doc.lineAt(slash.from).from, slash.from).trim()) return null;
  const query = normalize(slash.text.slice(1).trim());
  return {
    from: slash.from,
    filter: false,
    options: slashCommands
      .filter((c) => !query || normalize(c.label + " " + c.keywords).includes(query))
      .map((c): Completion => ({
        label: c.label,
        detail: c.detail,
        type: "keyword",
        apply: (editor, _c, from, to) => {
          editor.dispatch({ changes: { from, to }, selection: { anchor: from } });
          c.run(editor);
        },
      })),
  };
}

// ---------- Imágenes ----------

/** Guarda las imágenes y las inserta, cada una en su propia línea, en `at` o en el cursor. */
async function insertImages(files: File[], at?: number) {
  if (!view || !files.some(isImageFile)) return;
  uploading.value = true;
  const id = currentId;
  try {
    const refs = await uploadImages(files, (text) => emit("notify", "error", text));
    // Si el usuario cambió de nota mientras se subía, no insertar en la otra.
    if (!view || currentId !== id || !refs.length) return;
    const { state } = view;
    const nl = state.lineBreak;
    const pos = Math.min(at ?? state.selection.main.head, state.doc.length);
    const line = state.doc.lineAt(pos);
    const before = state.sliceDoc(line.from, pos).trim() ? nl : "";
    const after = state.sliceDoc(pos, line.to).trim() ? nl : "";
    const insert = before + refs.map(serializeImage).join(nl) + after;
    view.dispatch({ changes: { from: pos, insert }, selection: { anchor: pos + insert.length - after.length }, scrollIntoView: true });
  } finally {
    uploading.value = false;
    view?.focus();
  }
}

async function pickImages() {
  await insertImages(await pickImageFiles(true));
}

const hasImage = (list: FileList | undefined | null) => !!list && Array.from(list).some(isImageFile);

// ---------- Enlaces ----------

/** Rango del enlace bajo `el` y su destino, o null si no es un enlace. */
function linkAt(editor: EditorView, el: HTMLElement): { from: number; to: number; note?: string; url?: string } | null {
  const noteEl = el.closest<HTMLElement>("[data-note-link]");
  const urlEl = el.closest<HTMLElement>("[data-external-link]");
  if (!noteEl && !urlEl) return null;
  const pos = editor.posAtDOM(noteEl ?? urlEl!);
  if (noteEl) {
    const line = editor.state.doc.lineAt(pos);
    for (const m of line.text.matchAll(/\[\[[^\]\n]+\]\]/g)) {
      const from = line.from + m.index!;
      const to = from + m[0].length;
      if (pos >= from && pos <= to) return { from, to, note: noteEl.dataset.noteLink };
    }
    return { from: pos, to: pos, note: noteEl.dataset.noteLink };
  }
  let node = syntaxTree(editor.state).resolveInner(pos, 1);
  while (node.parent && node.name !== "Link") node = node.parent;
  return { from: node.from, to: node.to, url: urlEl!.dataset.externalLink };
}

function followLink(link: { note?: string; url?: string }) {
  if (link.note) return emit("wikiLinkClick", link.note);
  try {
    const url = new URL(link.url!);
    if (["http:", "https:", "mailto:"].includes(url.protocol)) void openUrl(url.href);
  } catch {
    emit("notify", "info", "El enlace está incompleto");
  }
}

// ---------- Barra flotante y botón «+» ----------

function updateBubble() {
  if (!view) return;
  const sel = view.state.selection.main;
  if (sel.empty || !view.hasFocus || props.sourceMode) {
    bubble.visible = false;
    return;
  }
  view.requestMeasure({
    read: (v) => ({ start: v.coordsAtPos(sel.from), end: v.coordsAtPos(sel.to) }),
    write: ({ start, end }) => {
      if (!start || !end) return void (bubble.visible = false);
      const above = start.top - 46;
      bubble.x = Math.max(8, Math.min(start.left, window.innerWidth - 340));
      bubble.y = above > 56 ? above : end.bottom + 8;
      bubble.visible = true;
    },
  });
}

function onMouseMove(e: MouseEvent) {
  if (!view || !wrapper.value || props.sourceMode) return;
  const pos = view.posAtCoords({ x: Math.max(e.clientX, view.contentDOM.getBoundingClientRect().left + 4), y: e.clientY }, false);
  const line = view.state.doc.lineAt(pos);
  const coords = view.coordsAtPos(line.from, 1);
  if (!coords) return;
  const top = wrapper.value.getBoundingClientRect().top;
  plus.line = line.number;
  plus.top = coords.top - top + (coords.bottom - coords.top) / 2 - 12;
  plus.visible = true;
}

function onPlus() {
  if (view) openInsertMenuAt(view, plus.line);
}

// ---------- Menú contextual ----------

function blockItems(editor: EditorView): MenuEntry[] {
  const current = currentBlock(editor);
  const item = (label: string, kind: BlockKind) => ({ label, checked: current === kind, action: () => setBlock(editor, kind) });
  return [
    item("Texto", "paragraph"),
    item("Título 1", "h1"),
    item("Título 2", "h2"),
    item("Título 3", "h3"),
    "separator",
    item("Lista", "bullet"),
    item("Lista numerada", "ordered"),
    item("Tarea", "task"),
    item("Cita", "quote"),
  ];
}

function editorMenu(event: MouseEvent, editor: EditorView) {
  const target = event.target as HTMLElement;
  // El contenedor de imagen y las tablas tienen su propio menú.
  if (target.closest(".cm-image-host, .editable-table")) return false;
  const clicked = editor.posAtCoords({ x: event.clientX, y: event.clientY });
  const sel = editor.state.selection.main;
  // Como en cualquier editor: clic derecho fuera de la selección mueve el cursor.
  if (clicked !== null && (sel.empty || clicked < sel.from || clicked > sel.to)) editor.dispatch({ selection: { anchor: clicked } });
  const hasSel = !editor.state.selection.main.empty;
  const link = linkAt(editor, target);

  const items: MenuEntry[] = [];
  if (link?.note) items.push({ label: `Abrir «${link.note}»`, action: () => followLink(link) }, "separator");
  if (link?.url)
    items.push(
      { label: "Abrir enlace en el navegador", action: () => followLink(link) },
      { label: "Copiar dirección del enlace", action: () => navigator.clipboard.writeText(link.url!) },
      "separator",
    );
  items.push(
    { label: "Cortar", shortcut: "Ctrl+X", disabled: !hasSel, action: () => copySelection(editor, true) },
    { label: "Copiar", shortcut: "Ctrl+C", disabled: !hasSel, action: () => copySelection(editor) },
    {
      label: "Pegar",
      shortcut: "Ctrl+V",
      action: async () => {
        if (!(await pasteText(editor))) emit("notify", "info", "El sistema no deja pegar desde el menú; usa Ctrl+V");
      },
    },
    {
      label: "Seleccionar todo",
      shortcut: "Ctrl+A",
      action: () => (editor.dispatch({ selection: { anchor: 0, head: editor.state.doc.length } }), editor.focus()),
    },
    "separator",
    {
      label: "Formato",
      children: [
        { label: "Negrita", shortcut: "Ctrl+B", action: () => toggleWrap(editor, "**") },
        { label: "Cursiva", shortcut: "Ctrl+I", action: () => toggleWrap(editor, "*") },
        { label: "Tachado", action: () => toggleWrap(editor, "~~") },
        { label: "Código", shortcut: "Ctrl+E", action: () => toggleWrap(editor, "`") },
        "separator",
        { label: "Enlace", shortcut: "Ctrl+K", action: () => insertLink(editor) },
        { label: "Enlace a nota", shortcut: "[[", action: () => insertWikiLink(editor) },
      ],
    },
    { label: "Convertir en", children: blockItems(editor) },
    {
      label: "Insertar",
      children: [
        { label: uploading.value ? "Subiendo imagen…" : "Imagen…", disabled: uploading.value, action: () => pickImages() },
        { label: "Tabla", action: () => insertTemplate(editor, "table") },
        { label: "Bloque de código", action: () => insertTemplate(editor, "code") },
        { label: "Fórmula", action: () => insertTemplate(editor, "math") },
        { label: "Diagrama Mermaid", action: () => insertTemplate(editor, "mermaid") },
        { label: "Separador", action: () => insertTemplate(editor, "rule") },
      ],
    },
    "separator",
    { label: "Deshacer", shortcut: "Ctrl+Z", action: () => (undo(editor), editor.focus()) },
    { label: "Rehacer", shortcut: "Ctrl+Mayús+Z", action: () => (redo(editor), editor.focus()) },
  );
  menu.open(event, items);
  return true;
}

function openBlockMenu(event: MouseEvent) {
  if (!view) return;
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  menu.openAt(rect.left, rect.bottom + 4, blockItems(view));
}

// ---------- Salir de un bloque ----------
// Tablas, código, fórmulas, diagramas y separadores se pintan como widget: si
// no hay una línea debajo, el cursor no tiene dónde ir. Se deja una línea en
// blanco (en GFM, sin ella el texto sería otra fila de la tabla) y una línea
// vacía para seguir escribiendo.

/** Cambios para dejar una línea editable tras el bloque que acaba en la línea `lastNumber`. */
function lineAfterBlock(state: EditorState, lastNumber: number): TransactionSpec {
  const { doc, lineBreak: nl } = state;
  const last = doc.line(lastNumber);
  let blanks = 0;
  while (lastNumber + blanks < doc.lines && !doc.line(lastNumber + blanks + 1).text.trim()) blanks++;
  const hasMore = lastNumber + blanks < doc.lines;
  // Bloque · blanco · cursor · (blanco · lo que siga): sin el último blanco, lo
  // escrito se uniría al párrafo de abajo.
  const needed = hasMore ? 3 : 2;
  if (blanks >= needed) return { selection: { anchor: doc.line(lastNumber + 2).from }, scrollIntoView: true };
  return { changes: { from: last.to, insert: nl.repeat(needed - blanks) }, selection: { anchor: last.to + nl.length * 2 }, scrollIntoView: true };
}

/** Última línea no vacía en o antes de `pos`. */
function lastFilledLine(state: EditorState, pos: number) {
  let line = state.doc.lineAt(pos);
  while (line.number > 1 && !line.text.trim()) line = state.doc.line(line.number - 1);
  return line;
}

/** Si un bloque con widget termina en esa línea, la especificación para salir de él. */
function exitBlockAt(state: EditorState, pos: number): TransactionSpec | null {
  const line = lastFilledLine(state, pos);
  const ends = previewBlocks(state.sliceDoc()).some((b) => state.doc.lineAt(b.to).number === line.number);
  return ends ? lineAfterBlock(state, line.number) : null;
}

/** Tras pegar Markdown que termina en un bloque, el cursor queda en una línea nueva debajo. */
const exitPastedBlock = EditorState.transactionFilter.of((tr) => {
  if (!tr.docChanged || !tr.isUserEvent("input.paste")) return tr;
  let end = -1;
  tr.changes.iterChanges((_fromA, _toA, _fromB, toB, text) => {
    if (text.length) end = toB;
  });
  if (end < 0) return tr;
  const spec = exitBlockAt(tr.state, end);
  return spec ? [tr, { ...spec, sequential: true }] : tr;
});

/** Si la nota termina en un bloque, prepara la línea de debajo (clic bajo el texto). */
const trailingBlockSpec = (state: EditorState) => exitBlockAt(state, state.doc.length);

// Texto con dibujos de caja (árboles de carpetas, diagramas ASCII): sin
// valla de código, el Markdown lo deshace. Se pega ya dentro de ``` … ```.
const boxDrawing = /[\u2500-\u257F]/;
function looksLikeAsciiArt(text: string) {
  const lines = text.split(/\r?\n/).filter((l) => l.trim());
  return lines.length >= 2 && lines.filter((l) => boxDrawing.test(l)).length >= 2 && !/^\s*(```|~~~)/m.test(text);
}

function insideCode(state: EditorState, pos: number) {
  for (let node: SyntaxNode | null = syntaxTree(state).resolveInner(pos, -1); node; node = node.parent)
    if (["FencedCode", "CodeBlock", "InlineCode"].includes(node.name)) return true;
  return false;
}

function pasteAsCodeBlock(editor: EditorView, text: string) {
  const { state } = editor;
  const nl = state.lineBreak;
  const { from, to } = state.selection.main;
  const before = state.sliceDoc(state.doc.lineAt(from).from, from).trim() ? nl : "";
  const after = state.sliceDoc(to, state.doc.lineAt(to).to).trim() ? nl : "";
  const body = text.replace(/\r?\n/g, nl).replace(/\s+$/, "");
  const insert = `${before}\`\`\`${nl}${body}${nl}\`\`\`${after}`;
  editor.dispatch({
    changes: { from, to, insert },
    selection: { anchor: from + insert.length - after.length },
    userEvent: "input.paste",
    scrollIntoView: true,
  });
}

// ---------- Estado de CodeMirror ----------

function createState(id: string, content: string) {
  return EditorState.create({
    doc: content,
    extensions: [
      EditorState.lineSeparator.of(content.includes("\r\n") ? "\r\n" : "\n"),
      markdown({ extensions: [GFM] }),
      history(),
      drawSelection(),
      highlightActiveLine(),
      EditorView.lineWrapping,
      syntaxHighlighting(markdownHighlightStyle),
      placeholder("Escribe algo, o pulsa «/» para insertar títulos, listas, imágenes, tablas…"),
      liveDecorations,
      exitPastedBlock,
      autocompletion({ override: [completions], icons: false }),
      keymap.of([
        { key: "Mod-k", run: (v) => (insertLink(v), true) },
        { key: "Mod-b", run: (v) => (toggleWrap(v, "**"), true) },
        { key: "Mod-i", run: (v) => (toggleWrap(v, "*"), true) },
        { key: "Mod-e", run: (v) => (toggleWrap(v, "`"), true) },
        { key: "Mod-Shift-x", run: (v) => (toggleWrap(v, "~~"), true) },
        { key: "Mod-Alt-1", run: (v) => (setBlock(v, "h1"), true) },
        { key: "Mod-Alt-2", run: (v) => (setBlock(v, "h2"), true) },
        { key: "Mod-Alt-3", run: (v) => (setBlock(v, "h3"), true) },
        { key: "Mod-Alt-0", run: (v) => (setBlock(v, "paragraph"), true) },
        ...completionKeymap,
        ...markdownKeymap,
        ...defaultKeymap,
        ...historyKeymap,
        indentWithTab,
      ]),
      EditorView.contentAttributes.of({ "aria-label": "Contenido de la nota", "aria-multiline": "true", spellcheck: "true", lang: "es" }),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !restoring) emit("change", id, update.state.sliceDoc());
        if (update.selectionSet || update.docChanged || update.focusChanged || update.geometryChanged) updateBubble();
      }),
      EditorView.domEventHandlers({
        focus: (_e, editor) => void editor.dispatch({ effects: focusEffect.of(true) }),
        blur: (_e, editor) => {
          editor.dispatch({ effects: focusEffect.of(false) });
          bubble.visible = false;
        },
        paste: (event, editor) => {
          if (!hasImage(event.clipboardData?.files)) {
            const text = event.clipboardData?.getData("text/plain") ?? "";
            if (!looksLikeAsciiArt(text) || insideCode(editor.state, editor.state.selection.main.from)) return false;
            event.preventDefault();
            pasteAsCodeBlock(editor, text);
            return true;
          }
          event.preventDefault();
          void insertImages(Array.from(event.clipboardData!.files));
          return true;
        },
        drop: (event, editor) => {
          if (!hasImage(event.dataTransfer?.files)) return false;
          event.preventDefault();
          const pos = editor.posAtCoords({ x: event.clientX, y: event.clientY }) ?? undefined;
          void insertImages(Array.from(event.dataTransfer!.files), pos);
          return true;
        },
        // Un enlace ya renderizado se sigue con un clic (como en Obsidian y Notion);
        // si su Markdown está a la vista (cursor dentro), el clic edita. Ctrl+clic siempre lo sigue.
        mousedown: (event, editor) => {
          if (event.button !== 0) return false;
          const link = linkAt(editor, event.target as HTMLElement);
          if (!link) return false;
          const editing = editor.hasFocus && selectionTouches(editor.state, link.from, link.to);
          if (editing && !(event.ctrlKey || event.metaKey)) return false;
          event.preventDefault();
          followLink(link);
          return true;
        },
        contextmenu: (event, editor) => editorMenu(event, editor),
      }),
    ],
  });
}

function scrollElement() {
  return host.value?.closest<HTMLElement>(".main-canvas");
}

function remember() {
  if (view && currentId) sessions.set(currentId, { state: view.state, scroll: scrollElement()?.scrollTop || 0 });
}

/** Sincroniza CodeMirror con las props: cambio de nota, cambios externos y modo. */
function sync() {
  if (!view) return;
  restoring = true;
  try {
    if (currentId !== props.fileId) {
      remember();
      currentId = props.fileId;
      const cached = sessions.get(currentId);
      view.setState(cached?.state || createState(currentId, props.content));
      bubble.visible = plus.visible = false;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const el = scrollElement();
        if (el) el.scrollTop = cached?.scroll || 0;
      });
    }
    if (view.state.sliceDoc() !== props.content) {
      const anchor = Math.min(view.state.selection.main.anchor, props.content.length);
      const head = Math.min(view.state.selection.main.head, props.content.length);
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: props.content }, selection: { anchor, head }, annotations: external });
    }
    view.dispatch({ effects: [liveModeEffect.of(!props.sourceMode), focusEffect.of(view.hasFocus)] });
  } finally {
    restoring = false;
  }
}

const onCanvasScroll = () => updateBubble();
onMounted(() => {
  currentId = props.fileId;
  view = new EditorView({ state: createState(currentId, props.content), parent: host.value });
  sync();
  scrollElement()?.addEventListener("scroll", onCanvasScroll, { passive: true });
});
watch(() => [props.fileId, props.content, props.sourceMode], sync, { flush: "post" });
watch(
  () => props.openFileIds,
  (ids) => {
    for (const id of sessions.keys()) if (!ids.includes(id)) sessions.delete(id);
  },
);
onBeforeUnmount(() => {
  cancelAnimationFrame(frame);
  scrollElement()?.removeEventListener("scroll", onCanvasScroll);
  view?.destroy();
  sessions.clear();
});
defineExpose({
  focus: () => view?.focus(),
  /** Desplaza la vista para dejar `pos` arriba, sin mover el cursor (el esquema lo usa). */
  scrollToPos: (pos: number) => {
    if (!view) return;
    const at = Math.min(pos, view.state.doc.length);
    view.dispatch({ effects: EditorView.scrollIntoView(at, { y: "start", yMargin: 24 }) });
  },
  /** Coordenada vertical (en pantalla) de la línea que contiene `pos`. */
  topOf: (pos: number) => (view ? view.lineBlockAt(Math.min(pos, view.state.doc.length)).top + view.documentTop : null),
  focusEnd: () => {
    if (!view) return;
    view.focus();
    view.dispatch(trailingBlockSpec(view.state) ?? { selection: { anchor: view.state.doc.length }, scrollIntoView: true });
  },
});
</script>

<template>
  <div ref="wrapper" class="live-editor" :class="{ 'live-editor-source': sourceMode }" @mousemove="onMouseMove" @mouseleave="plus.visible = false">
    <button
      v-show="plus.visible && !sourceMode"
      type="button"
      class="line-plus"
      :style="{ top: plus.top + 'px' }"
      aria-label="Insertar bloque"
      title="Insertar bloque (o escribe «/»)"
      @mousedown.prevent
      @click="onPlus"
    >
      <Plus :size="16" aria-hidden="true" />
    </button>
    <div ref="host"></div>

    <Teleport to="body">
      <div
        v-show="bubble.visible"
        class="format-bubble"
        role="toolbar"
        aria-label="Formato de la selección"
        :style="{ left: bubble.x + 'px', top: bubble.y + 'px' }"
        @mousedown.prevent
      >
        <button type="button" class="bubble-text" title="Convertir en…" @click="openBlockMenu">Convertir en ▾</button>
        <span class="bubble-sep" aria-hidden="true"></span>
        <button type="button" aria-label="Negrita" title="Negrita (Ctrl+B)" @click="view && toggleWrap(view, '**')">
          <Bold :size="15" aria-hidden="true" />
        </button>
        <button type="button" aria-label="Cursiva" title="Cursiva (Ctrl+I)" @click="view && toggleWrap(view, '*')">
          <Italic :size="15" aria-hidden="true" />
        </button>
        <button type="button" aria-label="Tachado" title="Tachado (Ctrl+Mayús+X)" @click="view && toggleWrap(view, '~~')">
          <Strikethrough :size="15" aria-hidden="true" />
        </button>
        <button type="button" aria-label="Código" title="Código (Ctrl+E)" @click="view && toggleWrap(view, '`')"><Code :size="15" aria-hidden="true" /></button>
        <span class="bubble-sep" aria-hidden="true"></span>
        <button type="button" aria-label="Enlace" title="Enlace (Ctrl+K)" @click="view && insertLink(view)"><Link :size="15" aria-hidden="true" /></button>
        <button type="button" aria-label="Enlace a nota" title="Enlace a nota [[ ]]" @click="view && insertWikiLink(view)">
          <Link2 :size="15" aria-hidden="true" />
        </button>
      </div>
    </Teleport>
  </div>
</template>
