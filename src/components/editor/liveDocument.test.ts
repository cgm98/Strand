import { describe, expect, test } from "bun:test";
import { previewBlocks } from "./liveDocument";

describe("previewBlocks", () => {
  test("detecta una tabla de nivel superior", () => {
    const blocks = previewBlocks("| A | B |\n|---|---|\n| 1 | 2 |\n");
    expect(blocks).toHaveLength(1);
    expect(blocks[0].kind).toBe("table");
    expect(blocks[0].source).toBe("| A | B |\n|---|---|\n| 1 | 2 |");
  });

  // Regresión: una tabla dentro de una lista no generaba su widget (quedaba
  // como texto plano con «|») porque se descartaba todo lo que no fuera de
  // nivel 0. Ver MIGRACION.md / la conversación donde se reportó y arregló.
  test("detecta una tabla anidada dentro de un elemento de lista", () => {
    const source = "1. Paso con tabla:\n\n   | A | B |\n   |---|---|\n   | 1 | 2 |\n";
    const blocks = previewBlocks(source);
    const table = blocks.find((b) => b.kind === "table");
    expect(table).toBeDefined();
    expect(table!.source).toBe("   | A | B |\n   |---|---|\n   | 1 | 2 |");
    // El rango empieza y termina en límites de línea (lo exige la decoración
    // de bloque de CodeMirror), aunque la tabla esté indentada.
    expect(source.slice(table!.from, table!.from + 3)).toBe("   ");
  });

  test("un bloque de código dentro de una lista NO se convierte en bloque de vista previa (ya lo resalta el árbol de sintaxis de CodeMirror)", () => {
    const source = "1. Paso con código:\n   ```bash\n   echo hola\n   ```\n";
    const blocks = previewBlocks(source);
    expect(blocks.find((b) => b.kind === "code")).toBeUndefined();
  });

  test("un bloque de código de nivel superior sí se convierte en bloque de vista previa", () => {
    const blocks = previewBlocks("```bash\necho hola\n```\n");
    expect(blocks).toHaveLength(1);
    expect(blocks[0].kind).toBe("code");
  });

  test("un bloque mermaid anidado en una lista también se detecta", () => {
    const source = "1. Diagrama:\n\n   ```mermaid\n   graph TD; a-->b;\n   ```\n";
    const blocks = previewBlocks(source);
    expect(blocks.find((b) => b.kind === "mermaid")).toBeDefined();
  });

  test("una valla sin cerrar (el usuario sigue escribiendo) no se reemplaza todavía", () => {
    const blocks = previewBlocks("```bash\necho sin cerrar");
    expect(blocks).toHaveLength(0);
  });
});
