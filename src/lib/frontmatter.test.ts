import { describe, expect, test } from "bun:test";
import { parseFrontmatter } from "./frontmatter";

describe("parseFrontmatter", () => {
  test("extrae clave: valor y quita el frontmatter del cuerpo", () => {
    const { data, body } = parseFrontmatter("---\ntitle: Mi nota\ntags: [a, b]\n---\n# Hola\n");
    expect(data).toEqual({ title: "Mi nota", tags: "[a, b]" });
    expect(body).toBe("# Hola\n");
  });

  test("quita comillas simples o dobles alrededor del valor", () => {
    const { data } = parseFrontmatter("---\ntitle: \"Con comillas\"\nalt: 'Otra'\n---\ncuerpo");
    expect(data).toEqual({ title: "Con comillas", alt: "Otra" });
  });

  test("sin frontmatter: el cuerpo queda igual y data vacío", () => {
    const { data, body } = parseFrontmatter("# Sin frontmatter\ncontenido");
    expect(data).toEqual({});
    expect(body).toBe("# Sin frontmatter\ncontenido");
  });

  test("frontmatter sin cerrar se trata como si no existiera", () => {
    const { data, body } = parseFrontmatter("---\ntitle: roto\nsin cierre");
    expect(data).toEqual({});
    expect(body).toBe("---\ntitle: roto\nsin cierre");
  });

  test("una línea sin «:» se ignora sin romper el resto", () => {
    const { data } = parseFrontmatter("---\nesto no es clave-valor\ntitle: ok\n---\ncuerpo");
    expect(data).toEqual({ title: "ok" });
  });
});
