import { describe, expect, test } from "bun:test";
import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  test("títulos, negrita y código inline", () => {
    const html = renderMarkdown("# Título\n\nTexto con **negrita** y `código`.");
    expect(html).toContain("<h1>Título</h1>");
    expect(html).toContain("<strong>negrita</strong>");
    expect(html).toContain("<code>código</code>");
  });

  test("enlace [[nota]] se convierte en un enlace propio con data-note-link", () => {
    const html = renderMarkdown("Ver [[Otra nota]].");
    expect(html).toContain('data-note-link="Otra nota"');
    expect(html).toContain(">Otra nota<");
  });

  test("enlace [[nota|alias]] usa el alias como texto visible", () => {
    const html = renderMarkdown("Ver [[Otra nota|así]].");
    expect(html).toContain('data-note-link="Otra nota"');
    expect(html).toContain(">así<");
  });

  test("tareas: la casilla queda deshabilitada (de solo lectura)", () => {
    const html = renderMarkdown("- [ ] Pendiente\n- [x] Hecha\n");
    expect(html).toContain('<input type="checkbox" disabled>');
    expect(html).toContain('<input type="checkbox" disabled checked>');
  });

  test("imagen attachments/… se reescribe al protocolo attachment://", () => {
    const html = renderMarkdown("![alt](attachments/foo.png)");
    expect(html).toContain('src="attachment://foo.png"');
  });

  test("math inline con $…$ se renderiza con KaTeX", () => {
    const html = renderMarkdown("La fórmula $x^2$ es cuadrática.");
    expect(html).toContain("katex");
  });

  // Regresión del bug reportado: una tabla GFM dentro de una lista numerada no
  // se perdía al parsear (ver liveDocument.test.ts para la vista en vivo del
  // editor; esto solo cubre el render final).
  test("tabla GFM dentro de un elemento de lista se renderiza como <table>", () => {
    const html = renderMarkdown("1. Paso con tabla:\n\n   | A | B |\n   |---|---|\n   | 1 | 2 |\n");
    expect(html).toContain("<table>");
    expect(html).toContain("<th>A</th>");
    expect(html).toContain("<td>1</td>");
  });
});
