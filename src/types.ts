/** Nota tal como la ve la interfaz (árbol, pestañas, editor). */
export interface FileItem {
  id: string;
  name: string;
  folderId: string | null;
  /** Cuerpo Markdown; vacío si la nota aún no se ha cargado completa. */
  content: string;
  updatedAt?: number;
}

export interface FolderItem {
  id: string;
  name: string;
  parentId: string | null;
  version: number;
}

export interface TabItem {
  id: string;
  fileId: string;
}

export type SaveStatus = "saved" | "saving" | "dirty" | "error";
