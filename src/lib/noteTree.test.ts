import { describe, expect, test } from "bun:test";
import { folderPath, isWithin, treeRows } from "./noteTree";
import type { FileItem, FolderItem } from "../types";

const folder = (id: string, name: string, parentId: string | null): FolderItem => ({ id, name, parentId, version: 1 });
const file = (id: string, name: string, folderId: string | null): FileItem => ({ id, name, folderId, content: "" });

const folders: FolderItem[] = [folder("rust", "Rust", null), folder("tauri", "Tauri", "rust"), folder("web", "Web", null)];
const files: FileItem[] = [file("n1", "Notas sueltas", null), file("n2", "Comandos IPC", "tauri"), file("n3", "Hooks", "web")];

describe("treeRows", () => {
  test("sin filtro ni carpetas expandidas: raíz solo muestra sus hijos directos", () => {
    const rows = treeRows(folders, files, new Set());
    expect(rows.map((r) => r.id)).toEqual(["rust", "web", "n1"]);
  });

  test("expandir una carpeta revela sus hijos, incluida la subcarpeta anidada", () => {
    const rows = treeRows(folders, files, new Set(["rust", "tauri"]));
    expect(rows.map((r) => r.id)).toEqual(["rust", "tauri", "n2", "web", "n1"]);
  });

  test("el contador de una carpeta suma las notas de sus subcarpetas", () => {
    const rows = treeRows(folders, files, new Set());
    expect(rows.find((r) => r.id === "rust")?.count).toBe(1);
  });

  test("el filtro por texto expande automáticamente las ramas con coincidencias", () => {
    const rows = treeRows(folders, files, new Set(), "ipc");
    expect(rows.map((r) => r.id)).toEqual(["rust", "tauri", "n2"]);
  });

  test("el filtro no muestra ramas sin coincidencias", () => {
    const rows = treeRows(folders, files, new Set(), "ningún resultado");
    expect(rows).toEqual([]);
  });

  test("un padre que no existe (carpeta borrada) se trata como raíz, no se pierde ni cicla", () => {
    const orphan = [...folders, folder("huerfana", "Huérfana", "no-existe")];
    const rows = treeRows(orphan, files, new Set());
    expect(rows.map((r) => r.id)).toContain("huerfana");
  });

  test("un id de carpeta duplicado que formaría un ciclo no cuelga la función (se corta con `seen`)", () => {
    // "a" aparece dos veces: una como hija de la raíz y otra, más abajo, como
    // hija de "b" (que a su vez es hija de "a"). Sin el corte por `seen`,
    // visit() reentraría en "a" -> "b" -> "a" -> "b" -> … sin parar nunca.
    const corrupted = [folder("a", "A", null), folder("b", "B", "a"), folder("a", "A-dup", "b")];
    const rows = treeRows(corrupted, [], new Set(["a", "b"]));
    // Termina (no cuelga) y la segunda "a" aparece una vez, sin volver a
    // descender en ella (su propio hijo "b" ya se visitó más arriba).
    expect(rows.map((r) => r.name)).toEqual(["A", "B", "A-dup"]);
  });
});

describe("folderPath", () => {
  test("concatena los nombres desde la raíz hasta la carpeta", () => {
    expect(folderPath(folders, "tauri")).toEqual(["Rust", "Tauri"]);
  });
  test("la raíz (null) no tiene ruta", () => {
    expect(folderPath(folders, null)).toEqual([]);
  });
});

describe("isWithin", () => {
  test("una carpeta está dentro de sí misma", () => {
    expect(isWithin(folders, "rust", "rust")).toBe(true);
  });
  test("una subcarpeta está dentro de su ancestro", () => {
    expect(isWithin(folders, "tauri", "rust")).toBe(true);
  });
  test("carpetas sin relación no están una dentro de la otra", () => {
    expect(isWithin(folders, "web", "rust")).toBe(false);
  });
});
