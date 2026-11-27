import { describe, expect, test } from "bun:test";
import { parseMarkdownTable, pasteTableCells, serializeMarkdownTable, splitTableRow } from "./markdownTable";

describe("splitTableRow", () => {
  test("separa celdas y quita las barras de los extremos", () => {
    expect(splitTableRow("| A | B |")).toEqual(["A", "B"]);
  });
  test("una barra escapada (\\|) no separa celdas", () => {
    expect(splitTableRow("| A\\|B | C |")).toEqual(["A|B", "C"]);
  });
  test("ignora la indentación de una tabla anidada en una lista", () => {
    expect(splitTableRow("   | A | B |")).toEqual(["A", "B"]);
  });
});

describe("parseMarkdownTable", () => {
  test("lee encabezado, alineación y filas", () => {
    const table = parseMarkdownTable("| A | B | C |\n|:---|---:|:---:|\n| 1 | 2 | 3 |");
    expect(table.rows).toEqual([
      ["A", "B", "C"],
      ["1", "2", "3"],
    ]);
    expect(table.alignments).toEqual(["left", "right", "center"]);
  });

  test("rellena con celdas vacías si una fila tiene menos columnas que el encabezado", () => {
    const table = parseMarkdownTable("| A | B |\n|---|---|\n| solo uno |");
    expect(table.rows[1]).toEqual(["solo uno", ""]);
  });
});

describe("serializeMarkdownTable <-> parseMarkdownTable", () => {
  test("round-trip: serializar y volver a parsear da la misma tabla", () => {
    const original = parseMarkdownTable("| A | B |\n|:---:|---|\n| 1 | 2 |\n| 3 | 4 |");
    const again = parseMarkdownTable(serializeMarkdownTable(original));
    expect(again).toEqual(original);
  });

  test("escapa las barras «|» dentro de una celda al serializar", () => {
    const md = serializeMarkdownTable({ rows: [["A|B"]], alignments: [null] });
    expect(md).toBe("| A\\|B |\n| --- |");
  });
});

describe("pasteTableCells", () => {
  test("pega texto separado por tabs/saltos de línea en varias celdas", () => {
    const table = parseMarkdownTable("| A | B |\n|---|---|\n|  |  |");
    const pasted = pasteTableCells(table, 1, 0, "1\t2\n3\t4");
    expect(pasted.rows).toEqual([
      ["A", "B"],
      ["1", "2"],
      ["3", "4"],
    ]);
  });

  test("agranda la tabla si el texto pegado tiene más columnas que las que había", () => {
    const table = parseMarkdownTable("| A |\n|---|\n|  |");
    const pasted = pasteTableCells(table, 0, 0, "x\ty");
    expect(pasted.alignments).toHaveLength(2);
    expect(pasted.rows[0]).toEqual(["x", "y"]);
  });
});
