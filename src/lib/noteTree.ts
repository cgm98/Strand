// Portado de cogni-vault (`treeRows`). Se quitó `noteHierarchy`: la taxonomía
// (cerebros, áreas, dominios, temas) ya son carpetas normales.
import type { FileItem, FolderItem } from "../types";

export interface TreeRow {
  id: string;
  name: string;
  depth: number;
  folder?: FolderItem;
  /** Notas dentro de la carpeta, contando subcarpetas (y el filtro activo). */
  count?: number;
}

const collator = new Intl.Collator("es", { numeric: true, sensitivity: "base" });

/** Aplana el árbol en filas visibles según las carpetas expandidas y el filtro. */
export function treeRows(folders: FolderItem[], files: FileItem[], expanded: Set<string>, query = ""): TreeRow[] {
  const q = query.trim().toLocaleLowerCase();
  const ids = new Set(folders.map((f) => f.id));
  const parentOf = (id: string | null) => (id && ids.has(id) ? id : "root");
  const sorted = <T extends { name: string }>(items: T[]) => [...items].sort((a, b) => collator.compare(a.name, b.name));

  function visit(id: string, depth: number, ancestorMatch: boolean, seen: Set<string>): { rows: TreeRow[]; count: number } {
    if (seen.has(id)) return { rows: [], count: 0 };
    const nextSeen = new Set(seen).add(id);
    const result: TreeRow[] = [];
    let count = 0;
    for (const folder of sorted(folders.filter((f) => parentOf(f.parentId) === id))) {
      const match = ancestorMatch || (!!q && folder.name.toLocaleLowerCase().includes(q));
      const children = visit(folder.id, depth + 1, match, nextSeen);
      count += children.count;
      if (!q || match || children.rows.length) {
        result.push({ id: folder.id, name: folder.name, depth, folder, count: children.count });
        if (q || expanded.has(folder.id)) result.push(...children.rows);
      }
    }
    for (const file of sorted(files.filter((f) => parentOf(f.folderId) === id))) {
      if (!q || ancestorMatch || file.name.toLocaleLowerCase().includes(q)) {
        count++;
        result.push({ id: file.id, name: file.name, depth });
      }
    }
    return { rows: result, count };
  }

  return visit("root", 0, false, new Set()).rows;
}

/** Ruta legible de una carpeta: «Proyectos / Rust / Tauri». */
export function folderPath(folders: FolderItem[], id: string | null): string[] {
  const names: string[] = [];
  const seen = new Set<string>();
  while (id && !seen.has(id)) {
    seen.add(id);
    const folder = folders.find((f) => f.id === id);
    if (!folder) break;
    names.unshift(folder.name);
    id = folder.parentId;
  }
  return names;
}

/** `true` si `id` es `ancestor` o está dentro de él. */
export function isWithin(folders: FolderItem[], id: string | null, ancestor: string): boolean {
  const seen = new Set<string>();
  while (id && !seen.has(id)) {
    if (id === ancestor) return true;
    seen.add(id);
    id = folders.find((f) => f.id === id)?.parentId ?? null;
  }
  return false;
}
