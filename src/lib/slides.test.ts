import { describe, expect, test } from "bun:test";
import { splitSlides } from "./slides";

describe("splitSlides", () => {
  test("sin separador: toda la nota es una sola diapositiva", () => {
    expect(splitSlides("# Título\n\nTexto.")).toEqual(["# Título\n\nTexto."]);
  });

  test("un --- de nivel superior separa dos diapositivas", () => {
    expect(splitSlides("# Uno\n\n---\n\n# Dos")).toEqual(["# Uno", "# Dos"]);
  });

  test("varios --- separan varias diapositivas, en orden", () => {
    expect(splitSlides("A\n\n---\n\nB\n\n---\n\nC")).toEqual(["A", "B", "C"]);
  });

  test("un --- dentro de una cita (> ---) no separa diapositivas", () => {
    const source = "Antes\n\n> algo\n> ---\n> más\n\nDespués";
    expect(splitSlides(source)).toEqual([source.trim()]);
  });

  test("--- al principio o al final no deja diapositivas vacías", () => {
    expect(splitSlides("---\n\nContenido\n\n---")).toEqual(["Contenido"]);
  });

  test("--- consecutivos no generan una diapositiva vacía entre ellos", () => {
    expect(splitSlides("A\n\n---\n\n---\n\nB")).toEqual(["A", "B"]);
  });

  test("un documento vacío o solo con --- devuelve una única diapositiva (el texto tal cual)", () => {
    expect(splitSlides("")).toEqual([""]);
    expect(splitSlides("---")).toEqual(["---"]);
  });
});
