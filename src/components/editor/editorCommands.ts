// Comandos de edición compartidos por la barra flotante, el menú «/», el botón
// «+» y el menú contextual. Todos operan sobre el Markdown del documento.
import type { EditorView } from "@codemirror/view";
import { EditorSelection } from "@codemirror/state";
import { startCompletion } from "@codemirror/autocomplete";

export type BlockKind = "paragraph" | "h1" | "h2" | "h3" | "bullet" | "ordered" | "task" | "quote";

const BLOCK_PREFIX_RE = /^(\s*)(#{1,6}\s+|>\s?|[-*+]\s+\[[ xX]\]\s+|[-*+]\s+|\d+[.)]\s+)?/;

function prefixFor(kind: BlockKind, index: number): string {
  switch (kind) {
    case "h1":
      return "# ";
    case "h2":
      return "## ";
    case "h3":
      return "### ";
    case "bullet":
      return "- ";
    case "ordered":
      return `${index + 1}. `;
    case "task":
      return "- [ ] ";
    case "quote":
      return "> ";
    default:
      return "";
  }
}

/** Tipo de bloque de la línea del cursor (para marcar la opción activa). */
export function currentBlock(view: EditorView): BlockKind {
  const text = view.state.doc.lineAt(view.state.selection.main.head).text;
  if (/^\s*###\s/.test(text)) return "h3";
  if (/^\s*##\s/.test(text)) return "h2";
  if (/^\s*#\s/.test(text)) return "h1";
  if (/^\s*[-*+]\s+\[[ xX]\]\s/.test(text)) return "task";
  if (/^\s*[-*+]\s/.test(text)) return "bullet";
  if (/^\s*\d+[.)]\s/.test(text)) return "ordered";
  if (/^\s*>/.test(text)) return "quote";
  return "paragraph";
}

/** Convierte las líneas seleccionadas al tipo de bloque indicado (o lo quita si ya lo eran). */
export function setBlock(view: EditorView, kind: BlockKind) {
  const { state } = view;
  const toggleOff = kind !== "paragraph" && currentBlock(view) === kind;
  const changes = [];
  const seen = new Set<number>();
  let index = 0;
  for (const range of state.selection.ranges) {
    for (let n = state.doc.lineAt(range.from).number; n <= state.doc.lineAt(range.to).number; n++) {
      if (seen.has(n)) continue;
      seen.add(n);
      const line = state.doc.line(n);
      const match = BLOCK_PREFIX_RE.exec(line.text)!;
      const indent = match[1];
      const existing = match[2] ?? "";
      const prefix = toggleOff ? "" : prefixFor(kind, index++);
      changes.push({ from: line.from + indent.length, to: line.from + indent.length + existing.length, insert: prefix });
    }
  }
  view.dispatch({ changes, scrollIntoView: true });
  view.focus();
}

/** Pone o quita un delimitador alrededor de la selección (**negrita**, *cursiva*…). */
export function toggleWrap(view: EditorView, delimiter: string) {
  const { state } = view;
  view.dispatch(
    state.changeByRange((range) => {
      const len = delimiter.length;
      const before = state.sliceDoc(range.from - len, range.from);
      const after = state.sliceDoc(range.to, range.to + len);
      if (before === delimiter && after === delimiter) {
        return {
          changes: [
            { from: range.from - len, to: range.from },
            { from: range.to, to: range.to + len },
          ],
          range: EditorSelection.range(range.from - len, range.to - len),
        };
      }
      const text = state.sliceDoc(range.from, range.to);
      if (text.length > 2 * len && text.startsWith(delimiter) && text.endsWith(delimiter)) {
        return {
          changes: { from: range.from, to: range.to, insert: text.slice(len, -len) },
          range: EditorSelection.range(range.from, range.to - 2 * len),
        };
      }
      return {
        changes: { from: range.from, to: range.to, insert: delimiter + text + delimiter },
        range: EditorSelection.range(range.from + len, range.to + len),
      };
    }),
  );
  view.focus();
}

export function insertLink(view: EditorView) {
  const { from, to } = view.state.selection.main;
  const label = view.state.sliceDoc(from, to) || "enlace";
  const text = `[${label}](https://)`;
  // Deja seleccionado «https://» para escribir la URL directamente.
  view.dispatch({ changes: { from, to, insert: text }, selection: { anchor: from + label.length + 3, head: from + text.length - 1 } });
  view.focus();
}

/** `[[selección]]`, o abre el autocompletado de notas si no hay selección. */
export function insertWikiLink(view: EditorView) {
  const { from, to } = view.state.selection.main;
  const label = view.state.sliceDoc(from, to);
  if (label) {
    view.dispatch({ changes: { from, to, insert: `[[${label}]]` }, selection: { anchor: from + label.length + 4 } });
  } else {
    view.dispatch({ changes: { from, insert: "[[]]" }, selection: { anchor: from + 2 } });
    startCompletion(view);
  }
  view.focus();
}

/**
 * Inserta un bloque de varias líneas en su propia línea. Si la línea actual
 * está vacía la ocupa; si no, va debajo. `cursorOffset` coloca el cursor dentro.
 */
export function insertBlock(view: EditorView, text: string, cursorOffset = text.length) {
  const { state } = view;
  const nl = state.lineBreak;
  const body = text.replace(/\n/g, nl);
  const line = state.doc.lineAt(state.selection.main.head);
  const empty = !line.text.trim();
  const from = empty ? line.from : line.to;
  const insert = (empty ? "" : nl) + body;
  // `cursorOffset` cuenta en «\n»; el documento puede usar «\r\n».
  const offset = (empty ? 0 : nl.length) + text.slice(0, cursorOffset).replace(/\n/g, nl).length;
  view.dispatch({ changes: { from, to: line.to, insert }, selection: { anchor: from + offset }, scrollIntoView: true });
  view.focus();
}

export const BLOCK_TEMPLATES = {
  table: { text: "| Columna | Valor |\n| --- | --- |\n|  |  |", cursor: 2 },
  code: { text: "```\n\n```", cursor: 4 },
  math: { text: "$$\n\n$$", cursor: 3 },
  mermaid: { text: "```mermaid\nflowchart LR\n  A --> B\n```", cursor: 11 },
  rule: { text: "---\n", cursor: 4 },
} as const;

export function insertTemplate(view: EditorView, name: keyof typeof BLOCK_TEMPLATES) {
  const t = BLOCK_TEMPLATES[name];
  insertBlock(view, t.text, t.cursor);
}

/**
 * Botón «+» (como Notion): en una línea vacía abre el menú «/»; si no, crea
 * una línea nueva debajo y lo abre allí.
 */
export function openInsertMenuAt(view: EditorView, lineNumber: number) {
  const line = view.state.doc.line(lineNumber);
  const nl = view.state.lineBreak;
  if (!line.text.trim()) {
    view.dispatch({ changes: { from: line.from, to: line.to, insert: "/" }, selection: { anchor: line.from + 1 } });
  } else {
    view.dispatch({ changes: { from: line.to, insert: nl + "/" }, selection: { anchor: line.to + nl.length + 1 } });
  }
  view.focus();
  startCompletion(view);
}

// ---------- Portapapeles ----------

export async function copySelection(view: EditorView, cut = false) {
  const { from, to } = view.state.selection.main;
  if (from === to) return;
  await navigator.clipboard.writeText(view.state.sliceDoc(from, to));
  if (cut) view.dispatch({ changes: { from, to }, selection: { anchor: from } });
  view.focus();
}

/** Devuelve false si el sistema no deja leer el portapapeles (entonces, Ctrl+V). */
export async function pasteText(view: EditorView): Promise<boolean> {
  try {
    const text = await navigator.clipboard.readText();
    view.dispatch(view.state.replaceSelection(text));
    view.focus();
    return true;
  } catch {
    view.focus();
    return false;
  }
}
