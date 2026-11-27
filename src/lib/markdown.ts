// Portado de cogni-vault (frontend/src/lib/markdown.ts). Cambio: las imágenes
// `attachments/<nombre>` se resuelven al protocolo `attachment://` de Tauri.
import MarkdownIt from "markdown-it";
import katex from "katex";
import hljs from "highlight.js/lib/common";
import { attachmentUrl } from "./api";

export interface MarkdownDiagram {
  code: string;
  index: number;
}
export interface MarkdownResult {
  html: string;
  diagrams: MarkdownDiagram[];
}
export interface MathRange {
  from: number;
  to: number;
  content: string;
}

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
  highlight(code, language) {
    return language && hljs.getLanguage(language) ? hljs.highlight(code, { language, ignoreIllegals: true }).value : "";
  },
});
const escape = md.utils.escapeHtml;
// `errorColor` por defecto de KaTeX es #cc0000 fijo (nunca pasó por los temas):
// en Medianoche oscuro da ~3.2:1 contra el fondo, por debajo del 4.5:1 de WCAG
// AA. `--danger` sí está calibrado por tema (scripts/colores.py).
function math(source: string, displayMode: boolean): string {
  const html = katex.renderToString(source, { displayMode, throwOnError: false, trust: false, strict: "ignore", errorColor: "var(--danger)" });
  // En bloque, una fórmula rota merece la misma explicación que un diagrama
  // Mermaid roto (mensaje + motivo, ver MermaidRenderer.vue `.mermaid-error`),
  // no el fragmento rojo suelto de KaTeX sin decir qué pasó. En línea se deja
  // tal cual: ya trae su propio `title` con el motivo al pasar el ratón, y una
  // caja de alerta no cabe dentro de una frase.
  if (!displayMode || !html.includes('class="katex-error"')) return html;
  // El `title` que pone KaTeX ya viene escapado para atributo HTML (comprobado
  // con comillas/`<script>` en la fórmula); se reutiliza tal cual, sin volver
  // a escaparlo, para no duplicar entidades (`&amp;amp;`).
  const message = html.match(/class="katex-error" title="([^"]*)"/)?.[1]?.replace(/^ParseError:\s*/, "") ?? "No se pudo interpretar la fórmula.";
  return '<div class="note-math-error" role="alert"><strong>No se pudo interpretar esta fórmula</strong><p>' + message + "</p></div>\n";
}

export function inlineMathAt(source: string, position: number): MathRange | null {
  const escaped = (at: number) => {
    let n = 0;
    while (at > 0 && source[--at] === "\\") n++;
    return n % 2 === 1;
  };
  if (escaped(position)) return null;
  const opener = source.startsWith("\\(", position)
    ? "\\("
    : source[position] === "$" && source[position - 1] !== "$" && source[position + 1] !== "$"
      ? "$"
      : "";
  if (!opener) return null;
  const closer = opener === "$" ? "$" : "\\)";
  const start = position + opener.length;
  if (opener === "$" && /\s/.test(source[start] || " ")) return null;
  let end = source.indexOf(closer, start);
  while (end >= 0 && escaped(end)) end = source.indexOf(closer, end + closer.length);
  if (end < 0 || !source.slice(start, end).trim() || source.slice(start, end).includes("\n")) return null;
  if (opener === "$" && (/\s/.test(source[end - 1]) || /\d/.test(source[end + 1] || "") || source[end + 1] === "$")) return null;
  return { from: position, to: end + closer.length, content: source.slice(start, end) };
}

export function inlineMathRanges(source: string): MathRange[] {
  const ranges: MathRange[] = [];
  for (let i = 0; i < source.length; i++) {
    const match = inlineMathAt(source, i);
    if (match) {
      ranges.push(match);
      i = match.to - 1;
    }
  }
  return ranges;
}

md.inline.ruler.before("escape", "note_math", (state, silent) => {
  const match = inlineMathAt(state.src, state.pos);
  if (!match) return false;
  if (!silent) state.push("note_math", "", 0).content = match.content;
  state.pos = match.to;
  return true;
});
md.renderer.rules.note_math = (tokens, index) => math(tokens[index].content, false);

md.block.ruler.before(
  "fence",
  "note_math_block",
  (state, startLine, endLine, silent) => {
    if (state.sCount[startLine] - state.blkIndent >= 4) return false;
    const first = state.src.slice(state.bMarks[startLine] + state.tShift[startLine], state.eMarks[startLine]);
    const opener = first.startsWith("$$") ? "$$" : first.startsWith("\\[") ? "\\[" : "";
    if (!opener) return false;
    const closer = opener === "$$" ? "$$" : "\\]";
    const parts: string[] = [];
    let line = startLine;
    let fragment = first.slice(opener.length);
    for (;;) {
      const end = fragment.indexOf(closer);
      if (end >= 0 && !fragment.slice(end + closer.length).trim()) {
        parts.push(fragment.slice(0, end));
        break;
      }
      parts.push(fragment);
      if (++line >= endLine) return false; // Una fórmula sin cerrar se queda como texto.
      fragment = state.src.slice(state.bMarks[line] + state.tShift[line], state.eMarks[line]);
    }
    if (silent) return true;
    const token = state.push("note_math_block", "div", 0);
    token.block = true;
    token.content = parts.join("\n");
    token.map = [startLine, line + 1];
    state.line = line + 1;
    return true;
  },
  { alt: ["paragraph", "reference", "blockquote", "list"] },
);
// `tabindex="0"`: con el techo de alto de document.css (--block-max-height) este
// div puede quedar con scroll propio; sin esto no hay forma de desplazarlo solo
// con teclado (no es foco nativo de un <div>).
md.renderer.rules.note_math_block = (tokens, index) => '<div class="note-math" tabindex="0">' + math(tokens[index].content, true) + "</div>\n";

md.inline.ruler.before("link", "note_wikilink", (state, silent) => {
  // `linkLevel` existe en el runtime de markdown-it, pero @types/markdown-it no lo declara.
  const { linkLevel } = state as typeof state & { linkLevel: number };
  if (linkLevel > 0 || !state.src.startsWith("[[", state.pos)) return false;
  const end = state.src.indexOf("]]", state.pos + 2);
  if (end < 0) return false;
  const [target, label] = state.src.slice(state.pos + 2, end).split("|");
  if (!target.trim()) return false;
  if (!silent) {
    const token = state.push("note_wikilink", "", 0);
    token.content = label || target;
    token.meta = { target: target.trim() };
  }
  state.pos = end + 2;
  return true;
});
md.renderer.rules.note_wikilink = (tokens, index) =>
  '<a href="#" class="wikilink" data-note-link="' + escape(String(tokens[index].meta?.target || "")) + '">' + escape(tokens[index].content) + "</a>";

const defaultImage = md.renderer.rules.image!;
md.renderer.rules.image = (tokens, index, options, env, renderer) => {
  const token = tokens[index];
  const src = token.attrGet("src") || "";
  if (src.startsWith("attachments/")) token.attrSet("src", attachmentUrl(src));
  token.attrSet("loading", "lazy");
  return defaultImage(tokens, index, options, env, renderer);
};

const fence = md.renderer.rules.fence!;
md.renderer.rules.fence = (tokens, index, options, env, renderer) => {
  const token = tokens[index];
  const lang = token.info.trim().split(/\s+/)[0].toLowerCase();
  if (["math", "latex", "tex"].includes(lang)) return '<div class="note-math" tabindex="0">' + math(token.content, true) + "</div>\n";
  if (lang !== "mermaid" || !Array.isArray(env.diagrams)) return addPreTabIndex(fence(tokens, index, options, env, renderer));
  const diagram = { index: env.diagrams.length, code: token.content };
  env.diagrams.push(diagram);
  return '<div class="note-diagram" data-mermaid-index="' + diagram.index + '"></div>\n';
};

// Mismo motivo que en note_math_block: el <pre> de un bloque de código puede
// quedar con scroll propio (--block-max-height) y por defecto no es foco de
// teclado. markdown-it siempre abre el bloque con exactamente "<pre><code".
function addPreTabIndex(html: string): string {
  return html.startsWith("<pre><code") ? '<pre tabindex="0"><code' + html.slice("<pre><code".length) : html;
}
const codeBlock = md.renderer.rules.code_block!;
md.renderer.rules.code_block = (tokens, index, options, env, renderer) => addPreTabIndex(codeBlock(tokens, index, options, env, renderer));

// En lectura, las tareas son casillas desactivadas, nunca HTML editable.
md.core.ruler.after("inline", "note_tasks", (state) => {
  for (let i = 2; i < state.tokens.length; i++) {
    const token = state.tokens[i];
    if (token.type !== "inline" || state.tokens[i - 1].type !== "paragraph_open" || state.tokens[i - 2].type !== "list_item_open") continue;
    const first = token.children?.[0];
    const match = first?.type === "text" && first.content.match(/^\[([ xX])\]\s+/);
    if (!match || !first) continue;
    state.tokens[i - 2].attrJoin("class", "task-list-item");
    first.content = first.content.slice(match[0].length);
    // Dentro de un <label>, el texto de la tarea (con su formato: negrita,
    // enlaces…) le da nombre accesible a la casilla sin tener que extraerlo
    // a mano de los tokens hijos.
    const open = new state.Token("html_inline", "", 0);
    open.content = "<label>";
    const checkbox = new state.Token("html_inline", "", 0);
    checkbox.content = '<input type="checkbox" disabled' + (match[1].toLowerCase() === "x" ? " checked" : "") + "> ";
    const close = new state.Token("html_inline", "", 0);
    close.content = "</label>";
    token.children!.unshift(open, checkbox);
    token.children!.push(close);
  }
});

// El editor y la vista de lectura comparten tokens y mapas de líneas.
export function parseMarkdown(source = "") {
  return md.parse(source, {});
}
export function renderMarkdown(source = ""): string {
  return md.render(source);
}
export function renderMarkdownDocument(source = ""): MarkdownResult {
  const diagrams: MarkdownDiagram[] = [];
  return { html: md.render(source, { diagrams }), diagrams };
}
