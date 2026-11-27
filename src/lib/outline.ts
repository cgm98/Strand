// Esquema de una nota: sus títulos Markdown (# … ######, y los subrayados con
// === / ---) en orden, con su nivel y su posición en el texto. Se usa el mismo
// parser que el editor (Lezer + GFM), así que un «# » dentro de un bloque de
// código no cuenta como título.
import { parser as baseParser, GFM } from "@lezer/markdown";

export interface OutlineItem {
  /** Nivel Markdown: 1 para «#», 6 para «######». */
  level: number;
  text: string;
  /** Desplazamiento en el texto donde empieza la línea del título. */
  from: number;
}

const parser = baseParser.configure(GFM);

/** Texto legible de un título: sin marcas de formato ni sintaxis de enlaces. */
export function plainHeading(raw: string): string {
  return raw
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1") // imagen
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1") // enlace
    .replace(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g, (_m, target: string, alias?: string) => alias ?? target) // [[nota|alias]]
    .replace(/(\*\*|__|~~|==)(.+?)\1/g, "$2")
    .replace(/(\*|_)(.+?)\1/g, "$2")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\s+/g, " ")
    .trim();
}

export function extractOutline(markdown: string): OutlineItem[] {
  const items: OutlineItem[] = [];
  parser.parse(markdown).iterate({
    enter(node) {
      const match = /^(ATX|Setext)Heading([1-6])$/.exec(node.name);
      if (!match) return;
      // Quitar las marcas (los «#» de ATX y el subrayado de Setext).
      let text = "";
      let cursor = node.from;
      for (let child = node.node.firstChild; child; child = child.nextSibling) {
        if (child.name !== "HeaderMark") continue;
        text += markdown.slice(cursor, child.from);
        cursor = child.to;
      }
      text += markdown.slice(cursor, node.to);
      const clean = plainHeading(text);
      if (clean) items.push({ level: Number(match[2]), text: clean, from: node.from });
      return false;
    },
  });
  return items;
}
