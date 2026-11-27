// Divide una nota en diapositivas para el modo Presentación. Convención de
// Marp/reveal.js: cada `---` de nivel superior (una línea horizontal sola,
// no una dentro de una lista o una cita) separa una diapositiva de la
// siguiente. Sin ningún separador, toda la nota es una sola diapositiva.
import { parseMarkdown } from "./markdown";

export function splitSlides(source: string): string[] {
  const offsets = [0];
  for (const match of source.matchAll(/\r\n|\r|\n/g)) offsets.push(match.index! + match[0].length);

  const cuts: Array<[end: number, next: number]> = [];
  for (const token of parseMarkdown(source)) {
    if (token.type === "hr" && token.level === 0 && token.map) cuts.push([offsets[token.map[0]], offsets[token.map[1]] ?? source.length]);
  }

  const slides: string[] = [];
  let start = 0;
  for (const [end, next] of cuts) {
    slides.push(source.slice(start, end).trim());
    start = next;
  }
  slides.push(source.slice(start).trim());

  const nonEmpty = slides.filter(Boolean);
  return nonEmpty.length ? nonEmpty : [source];
}
