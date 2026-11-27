// Portado de cogni-vault. Cambios: saneado con `sanitizeHtml` (admite imágenes
// `attachment://`), widgets inline accesibles por teclado y rutas de import.
import { StateField, StateEffect, Transaction, type EditorState, type Range, type Text } from "@codemirror/state";
import { EditorView, Decoration, WidgetType, type DecorationSet } from "@codemirror/view";
import { isolateHistory } from "@codemirror/commands";
import { syntaxTree } from "@codemirror/language";
import MermaidRenderer from "../MermaidRenderer.vue";
import BlockHeader from "./BlockHeader.vue";
import { mountWidget, unmountWidget } from "./mountWidget";
import { useToast } from "../../composables/useToast";
import { renderMarkdown, inlineMathRanges } from "../../lib/markdown";
import { sanitizeHtml } from "../../lib/sanitize";
import { previewBlocks, selectionTouches } from "./liveDocument";
import { TableWidget } from "./tableWidget";
import { ImageWidget } from "./imageWidget";
import { parseImage } from "../../lib/images";

const blockCache = new WeakMap<Text, ReturnType<typeof previewBlocks>>();
export const liveModeEffect = StateEffect.define<boolean>();
export const focusEffect = StateEffect.define<boolean>();
const headerEls = new WeakMap<HTMLElement, HTMLElement>();
const previewMeta = new WeakMap<HTMLElement, { source: string; from: number; kind: string; inline: boolean }>();

// Mismo criterio que `imageWidget.ts#remove`: si el bloque ocupa líneas completas,
// borrar también el salto de línea de alrededor para no dejar una línea vacía
// huérfana (el pedido explícito de "no hay manera de borrar los bloques en blanco").
function removeBlock(view: EditorView, from: number, length: number) {
  const to = from + length;
  const startLine = view.state.doc.lineAt(from);
  const endLine = view.state.doc.lineAt(to);
  let delFrom = from,
    delTo = to;
  if (startLine.from === from && endLine.to === to) {
    delFrom = startLine.from;
    delTo = endLine.to;
    if (endLine.number < view.state.doc.lines) delTo += view.state.lineBreak.length;
    else if (startLine.number > 1) delFrom -= view.state.lineBreak.length;
  }
  view.dispatch({
    changes: { from: delFrom, to: delTo },
    selection: { anchor: delFrom },
    annotations: [Transaction.userEvent.of("delete.block"), isolateHistory.of("full")],
  });
  view.focus();
  useToast().info("Bloque eliminado. Ctrl+Z lo recupera.");
}

class PreviewWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly from: number,
    readonly kind: string,
    readonly code = "",
    readonly inline = false,
  ) {
    super();
  }
  eq(other: PreviewWidget) {
    return this.source === other.source && this.from === other.from && this.kind === other.kind && this.inline === other.inline;
  }
  toDOM(view: EditorView) {
    const dom = document.createElement(this.inline ? "span" : "div");
    previewMeta.set(dom, { source: this.source, from: this.from, kind: this.kind, inline: this.inline });
    dom.className = this.inline ? "cm-live-inline" : "cm-live-preview markdown-reading";
    const reveal = () => {
      view.dispatch({ selection: { anchor: previewMeta.get(dom)!.from }, scrollIntoView: true });
      view.focus();
    };
    const remove = () => removeBlock(view, previewMeta.get(dom)!.from, this.source.length);
    if (this.kind === "mermaid") {
      // MermaidRenderer trae su propia cabecera (Ampliar, Copiar, Editar, Eliminar):
      // ningún botón aparte, igual que si fuera un bloque de código normal.
      mountWidget(dom, MermaidRenderer, { code: this.code, onEdit: reveal, onRemove: remove });
      return dom;
    }
    dom.innerHTML = sanitizeHtml(renderMarkdown(this.source));
    // Un widget inline no puede contener un párrafo de bloque (rompe la línea).
    if (this.inline && dom.firstElementChild?.tagName === "P") dom.replaceChildren(...dom.firstElementChild.childNodes);
    if (this.inline) {
      dom.onclick = reveal;
      dom.title = this.kind === "image" ? "Editar imagen" : "Editar fórmula";
      if (this.kind === "math") dom.classList.add("cm-live-math-inline");
    } else if (this.kind === "code" || this.kind === "math") {
      // Cabecera estilo "bloque de código de chat": etiqueta + copiar/editar/eliminar
      // arriba a la derecha, en vez del botón de ancho completo de antes. La fórmula
      // en bloque usaba un botón suelto bajo el KaTeX; con esto queda consistente
      // con código y mermaid (misma tarjeta, mismas acciones).
      const isCode = this.kind === "code";
      dom.classList.add(isCode ? "cm-code-card" : "cm-math-card");
      const label = isCode ? (this.source.match(/^[`~]{3,}\s*(\S+)/)?.[1] ?? "") : "fórmula";
      const header = document.createElement("div");
      mountWidget(header, BlockHeader, {
        label,
        copyText: this.code,
        editTitle: isCode ? "Editar código" : "Editar fórmula",
        removeTitle: isCode ? "Eliminar código" : "Eliminar fórmula",
        onEdit: reveal,
        onRemove: remove,
      });
      headerEls.set(dom, header);
      dom.prepend(header);
    } else if (this.kind === "rule") {
      // Un separador no tiene contenido que editar: un botón de "Editar separador"
      // aparte era confuso y, con los márgenes de lectura del <hr>, el bloque
      // quedaba altísimo y vacío (parecía un bloque de código en blanco). Igual que
      // las fórmulas/imágenes inline: clic directo sobre la línea para revelar el
      // Markdown fuente.
      dom.classList.add("cm-rule-preview");
      dom.onclick = reveal;
      dom.title = "Editar separador";
    }
    return dom;
  }
  updateDOM(dom: HTMLElement) {
    const previous = previewMeta.get(dom);
    if (!previous || previous.source !== this.source || previous.kind !== this.kind || previous.inline !== this.inline) return false;
    previous.from = this.from;
    return true;
  }
  destroy(dom: HTMLElement) {
    previewMeta.delete(dom);
    unmountWidget(dom);
    const header = headerEls.get(dom);
    if (header) unmountWidget(header);
    headerEls.delete(dom);
  }
  ignoreEvent() {
    return true;
  }
}

class TaskWidget extends WidgetType {
  constructor(
    readonly from: number,
    readonly checked: boolean,
  ) {
    super();
  }
  eq(other: TaskWidget) {
    return other.from === this.from && other.checked === this.checked;
  }
  toDOM(view: EditorView) {
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = this.checked;
    input.setAttribute("aria-label", this.checked ? "Marcar tarea como pendiente" : "Completar tarea");
    input.onchange = () => view.dispatch({ changes: { from: this.from + 1, to: this.from + 2, insert: input.checked ? "x" : " " } });
    return input;
  }
  ignoreEvent() {
    return true;
  }
}

class BulletWidget extends WidgetType {
  toDOM() {
    const bullet = document.createElement("span");
    bullet.className = "cm-list-bullet";
    bullet.textContent = "•";
    return bullet;
  }
  ignoreEvent() {
    return false;
  }
}

export const liveDecorations = StateField.define<{ decorations: DecorationSet; enabled: boolean; focused: boolean }>({
  create(state) {
    return { decorations: build(state, false), enabled: true, focused: false };
  },
  update(value, tr) {
    let { enabled, focused } = value;
    for (const effect of tr.effects) {
      if (effect.is(liveModeEffect)) enabled = effect.value;
      if (effect.is(focusEffect)) focused = effect.value;
    }
    if (!tr.docChanged && !tr.selection && !tr.effects.length && syntaxTree(tr.state) === syntaxTree(tr.startState)) return value;
    return { enabled, focused, decorations: enabled ? build(tr.state, focused) : Decoration.none };
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
});

function build(state: EditorState, focused: boolean): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  const source = state.doc.toString();
  let blocks = blockCache.get(state.doc);
  if (!blocks) {
    blocks = previewBlocks(source);
    blockCache.set(state.doc, blocks);
  }
  const hidden: Array<[number, number]> = [];
  const add = (from: number, to: number, decoration: Decoration) => {
    if (to > from) ranges.push(decoration.range(from, to));
  };
  for (const block of blocks) {
    if (block.kind === "table") {
      add(block.from, block.to, Decoration.replace({ widget: new TableWidget(block.source, block.from, block.to), block: true }));
      hidden.push([block.from, block.to]);
      continue;
    }
    const widget = new PreviewWidget(block.source, block.from, block.kind, block.code);
    if (
      focused &&
      selectionTouches(state, block.from, block.to) &&
      (block.kind === "code" || block.kind === "rule" || block.kind === "mermaid" || block.kind === "math")
    ) {
      // En código, mermaid, fórmulas y separadores, mientras se edita se ve el texto
      // fuente tal cual (con aspecto de bloque de código): mostrar además el diagrama o
      // el KaTeX renderizado debajo quedaba descuadrado (el widget de Mermaid trae su
      // propia cabecera y diálogo, pensados para el tamaño final, no para convivir
      // apretado entre líneas del editor) y duplicaba los botones de editar/eliminar que
      // ya tiene la cabecera de la vista renderizada. Las líneas de cerco (```lang / ```
      // o $$ / $$) se marcan aparte para atenuarlas: siguen editables (no se pueden
      // ocultar) pero no deben competir visualmente con el contenido, y el resto imita
      // el recuadro de la versión renderizada (document.css) para que el tamaño/borde no
      // "salten" al entrar o salir del bloque.
      if (block.kind === "code" || block.kind === "mermaid" || block.kind === "math") {
        const first = state.doc.lineAt(block.from).number;
        const last = state.doc.lineAt(block.to).number;
        for (let n = first; n <= last; n++) {
          const classes = ["cm-live-code-line"];
          if (n === first || n === last) classes.push("cm-live-code-fence");
          if (n === first) classes.push("cm-live-code-edge-top");
          if (n === last) classes.push("cm-live-code-edge-bottom");
          ranges.push(Decoration.line({ class: classes.join(" ") }).range(state.doc.line(n).from));
        }
      }
    } else if (focused && selectionTouches(state, block.from, block.to)) {
      // Mientras se edita el bloque, el código queda editable y la vista previa debajo.
      ranges.push(Decoration.widget({ widget, block: true, side: 1 }).range(block.to));
    } else {
      add(block.from, block.to, Decoration.replace({ widget, block: true }));
      hidden.push([block.from, block.to]);
    }
  }
  const isHidden = (from: number, to: number) => hidden.some(([a, b]) => from >= a && to <= b);
  const active = (from: number, to: number) => focused && selectionTouches(state, from, to);
  const code: Array<[number, number]> = [];
  syntaxTree(state).iterate({
    enter(node) {
      const { from, to, name } = node;
      if (isHidden(from, to)) return false;
      if (["FencedCode", "CodeBlock", "InlineCode"].includes(name)) code.push([from, to]);
      if (/^ATXHeading[1-6]$/.test(name) || /^SetextHeading[12]$/.test(name))
        ranges.push(Decoration.line({ class: "cm-live-h" + name.slice(-1) }).range(state.doc.lineAt(from).from));
      if (name === "StrongEmphasis") add(from, to, Decoration.mark({ class: "cm-live-strong" }));
      if (name === "Emphasis") add(from, to, Decoration.mark({ class: "cm-live-em" }));
      if (name === "Strikethrough") add(from, to, Decoration.mark({ class: "cm-live-strike" }));
      if (name === "InlineCode") add(from, to, Decoration.mark({ class: "cm-live-code" }));
      if (name === "Link" || name === "Autolink") {
        const url = node.node.getChild("URL");
        add(from, to, Decoration.mark({ class: "cm-live-link", attributes: url ? { "data-external-link": state.sliceDoc(url.from, url.to) } : {} }));
      }
      // URL "pelada" (GFM Autolink, sin `[]()` ni `<>`): no tiene nodo contenedor,
      // así que el marcado va directo sobre el propio nodo URL.
      if (name === "URL" && !["Link", "Autolink"].includes(node.node.parent?.name ?? ""))
        add(from, to, Decoration.mark({ class: "cm-live-link", attributes: { "data-external-link": state.sliceDoc(from, to) } }));
      if (
        (name === "LinkMark" || name === "URL") &&
        node.node.parent?.name &&
        ["Link", "Autolink"].includes(node.node.parent.name) &&
        !active(node.node.parent.from, node.node.parent.to)
      )
        add(from, to, Decoration.replace({}));
      if (name === "Image") {
        const source = state.sliceDoc(from, to);
        // Contenedor de imagen (ver ImageBlock.vue). Con el cursor en el Markdown,
        // el Markdown queda editable y el contenedor va detrás; si no, lo sustituye.
        // Si el Markdown no se reconoce (sintaxis rara), vista previa simple.
        if (!parseImage(source)) {
          if (!active(from, to)) add(from, to, Decoration.replace({ widget: new PreviewWidget(source, from, "image", "", true) }));
        } else if (active(from, to)) ranges.push(Decoration.widget({ widget: new ImageWidget(source, from, to, true), side: 1 }).range(to));
        else add(from, to, Decoration.replace({ widget: new ImageWidget(source, from, to, false) }));
        return false;
      }
      if (name === "FencedCode" && !blocks!.some((block) => block.from === from)) {
        for (let n = state.doc.lineAt(from).number; n <= state.doc.lineAt(to).number; n++)
          ranges.push(Decoration.line({ class: "cm-live-code-line" }).range(state.doc.line(n).from));
        if (!active(from, to)) {
          const first = state.doc.lineAt(from);
          const last = state.doc.lineAt(to);
          add(first.from, first.to, Decoration.replace({}));
          if (last.from > first.from && /^\s*(`{3,}|~{3,})\s*$/.test(last.text)) add(last.from, last.to, Decoration.replace({}));
        }
      }
      if (name === "Blockquote") {
        for (let n = state.doc.lineAt(from).number; n <= state.doc.lineAt(to).number; n++)
          ranges.push(Decoration.line({ class: "cm-live-quote" }).range(state.doc.line(n).from));
      }
      if (["HeaderMark", "EmphasisMark", "CodeMark", "StrikethroughMark", "QuoteMark"].includes(name)) {
        const parent = node.node.parent;
        if (parent && !active(parent.from, parent.to) && !["FencedCode", "CodeBlock"].includes(parent.name)) add(from, to, Decoration.replace({}));
      }
      if (name === "TaskMarker" && !active(state.doc.lineAt(from).from, state.doc.lineAt(to).to))
        add(from, to, Decoration.replace({ widget: new TaskWidget(from, source.slice(from, to).toLowerCase() === "[x]") }));
      if (name === "ListMark" && !active(state.doc.lineAt(from).from, state.doc.lineAt(to).to)) {
        const task = /^\s*\[[ xX]\]/.test(source.slice(to, state.doc.lineAt(to).to));
        if (task) add(from, to, Decoration.replace({}));
        else if (/^[-+*]$/.test(source.slice(from, to))) add(from, to, Decoration.replace({ widget: new BulletWidget() }));
      }
    },
  });
  // Sintaxis propia de las notas, fuera de CommonMark; nunca dentro de código.
  const outsideCode = (from: number, to: number) =>
    !isHidden(from, to) && !code.some(([a, b]) => from < b && to > a) && !blocks!.some((b) => from >= b.from && to <= b.to);
  for (const match of source.matchAll(/\[\[([^\]\n]+)\]\]/g)) {
    const from = match.index!;
    const to = from + match[0].length;
    if (!outsideCode(from, to)) continue;
    add(from, to, Decoration.mark({ class: "cm-live-link", attributes: { "data-note-link": match[1].split("|")[0] } }));
    if (!active(from, to)) {
      const alias = match[1].indexOf("|");
      add(from, from + 2 + (alias >= 0 ? alias + 1 : 0), Decoration.replace({}));
      add(to - 2, to, Decoration.replace({}));
    }
  }
  for (const match of inlineMathRanges(source)) {
    if (outsideCode(match.from, match.to) && !active(match.from, match.to))
      add(match.from, match.to, Decoration.replace({ widget: new PreviewWidget(source.slice(match.from, match.to), match.from, "math", "", true) }));
  }
  return Decoration.set(ranges, true);
}
