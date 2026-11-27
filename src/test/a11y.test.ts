// Accesibilidad automatizada del HTML que generamos nosotros (axe-core sobre
// jsdom). Alcance real, para no prometer de más:
//  - Sí cubre: nombres accesibles, roles y atributos ARIA del marcado que
//    produce `renderMarkdown()` (lo mismo que usan los widgets de vista
//    previa del editor en vivo, ver `liveDecorations.ts`).
//  - No cubre: contraste de color ni nada que dependa de layout real (jsdom
//    no pinta la página) — eso ya lo auditan los temas con
//    `scripts/colores.py`, y la pasada manual con un lector de pantalla
//    (Orca) sigue pendiente en el hito 8.
//  - La regla "region" se desactiva aquí porque cada prueba inyecta un
//    fragmento suelto en <body> sin las marcas de la página real (que sí
//    tiene su `role="tabpanel"`, etc.); no es una regla de contenido.
import { describe, expect, test } from "bun:test";
import axe from "axe-core";
import { renderMarkdown } from "../lib/markdown";

const AXE_OPTIONS = { rules: { "color-contrast": { enabled: false }, region: { enabled: false } } };

async function violations(html: string) {
  document.body.innerHTML = html;
  const results = await axe.run(document.body, AXE_OPTIONS);
  return results.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.html) }));
}

describe("accesibilidad del HTML renderizado", () => {
  test("una nota «normal» (títulos, listas, tareas, tabla, enlaces) no tiene violaciones", async () => {
    const html = renderMarkdown(
      [
        "# Título",
        "",
        "Texto con [un enlace](https://example.com) y [[una nota]].",
        "",
        "- [ ] Pendiente",
        "- [x] Hecha con **negrita**",
        "",
        "| A | B |",
        "|---|---|",
        "| 1 | 2 |",
      ].join("\n"),
    );
    expect(await violations(html)).toEqual([]);
  });

  test("una tarea con formato (negrita, enlace, código) sigue teniendo nombre accesible completo", async () => {
    const html = renderMarkdown("- [ ] Revisar [el PR](https://example.com) con `cargo test`\n");
    expect(await violations(html)).toEqual([]);
    expect(html).toContain("<label>");
  });

  test("una imagen sin texto alternativo (![](...)) no viola image-alt (alt vacío es válido: decorativa)", async () => {
    const html = renderMarkdown("![](attachments/foo.png)");
    const found = await violations(html);
    expect(found.find((v) => v.id === "image-alt")).toBeUndefined();
  });
});
