import { describe, expect, test } from "bun:test";
import { extractOutline, plainHeading } from "./outline";

describe("extractOutline", () => {
  test("lista los títulos ATX en orden con su nivel y posición", () => {
    const md = "# Título\n\nTexto.\n\n## Subtítulo\n\n### Otro\n";
    const items = extractOutline(md);
    expect(items.map((i) => [i.level, i.text])).toEqual([
      [1, "Título"],
      [2, "Subtítulo"],
      [3, "Otro"],
    ]);
    expect(items[0].from).toBe(0);
  });

  test("un «#» dentro de un bloque de código no cuenta como título", () => {
    const md = "# Real\n\n```\n# No es título\n```\n";
    const items = extractOutline(md);
    expect(items).toHaveLength(1);
    expect(items[0].text).toBe("Real");
  });

  test("títulos Setext (subrayados con === o ---) también se detectan", () => {
    const md = "Título\n======\n\nSubtítulo\n---------\n";
    const items = extractOutline(md);
    expect(items.map((i) => i.level)).toEqual([1, 2]);
  });

  test("un título vacío (solo marcas) se descarta", () => {
    const md = "#\n\n# Con texto\n";
    const items = extractOutline(md);
    expect(items).toHaveLength(1);
  });
});

describe("plainHeading", () => {
  test("quita negrita, cursiva, código y tachado", () => {
    expect(plainHeading("**negrita** y _cursiva_ y `código` y ~~tachado~~")).toBe("negrita y cursiva y código y tachado");
  });

  test("un enlace [texto](url) deja solo el texto", () => {
    expect(plainHeading("Ver [este enlace](https://example.com) ahora")).toBe("Ver este enlace ahora");
  });

  test("una imagen ![alt](url) deja solo el alt", () => {
    expect(plainHeading("![captura](foo.png) de pantalla")).toBe("captura de pantalla");
  });

  test("un wikilink [[nota|alias]] deja el alias", () => {
    expect(plainHeading("Ver [[Otra nota|así]]")).toBe("Ver así");
  });

  test("un wikilink [[nota]] sin alias deja el nombre de la nota", () => {
    expect(plainHeading("Ver [[Otra nota]]")).toBe("Ver Otra nota");
  });
});
