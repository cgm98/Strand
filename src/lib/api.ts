// Wrappers tipados de los comandos IPC definidos en src-tauri/src/commands.rs.
// Los tipos reflejan src-tauri/src/models.rs (serde en camelCase).
// Si cambias una firma en Rust, cámbiala aquí también.

import { convertFileSrc, invoke as rawInvoke } from "@tauri-apps/api/core";

// ---------- Errores ----------

export type ErrorCode = "NOT_FOUND" | "VALIDATION" | "CONFLICT" | "DB" | "IO" | "ZIP" | "INTERNAL";

/** Error que devuelve el backend (`AppError` serializado como `{ code, message }`). */
export class ApiError extends Error {
  constructor(
    public readonly code: ErrorCode,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function isAppErrorPayload(e: unknown): e is { code: ErrorCode; message: string } {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

async function invoke<T>(cmd: string, args?: Parameters<typeof rawInvoke>[1]): Promise<T> {
  try {
    return await rawInvoke<T>(cmd, args);
  } catch (e) {
    if (isAppErrorPayload(e)) throw new ApiError(e.code, e.message);
    // Errores de Tauri (comando inexistente, argumentos mal nombrados…) llegan como string.
    throw new ApiError("INTERNAL", typeof e === "string" ? e : String(e));
  }
}

// ---------- Modelos ----------

export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  dbPath: string;
}

export interface Folder {
  id: string;
  parentId: string | null;
  name: string;
  sortOrder: number;
  version: number;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface FolderNode extends Folder {
  noteCount: number;
  children: FolderNode[];
}

export interface Note {
  id: string;
  folderId: string | null;
  title: string;
  body: string;
  version: number;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}

export interface NoteSummary {
  id: string;
  folderId: string | null;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateNote {
  title: string;
  body?: string;
  folderId?: string | null;
}

/** `expectedVersion` debe ser la `version` de la última respuesta; si no, `CONFLICT`. */
export interface UpdateNote {
  title?: string;
  body?: string;
  expectedVersion: number;
}

export interface NoteVersionSummary {
  id: string;
  noteId: string;
  title: string;
  createdAt: string;
}

export interface NoteVersion extends NoteVersionSummary {
  body: string;
}

export interface Tag {
  id: string;
  name: string;
  color: string | null;
  noteCount: number;
}

export interface SearchHit {
  id: string;
  folderId: string | null;
  title: string;
  updatedAt: string;
  /** Coincidencias entre U+E000 y U+E001; usa `highlight()` para mostrarlo. */
  snippet: string;
  rank: number;
}

/** `noteId` es `null` si la nota enlazada aún no existe. */
export interface OutgoingLink {
  title: string;
  noteId: string | null;
}

export interface TrashBatch {
  id: string;
  rootFolderId: string;
  rootName: string;
  itemCount: number;
  createdAt: string;
}

export interface TrashContents {
  notes: NoteSummary[];
  batches: TrashBatch[];
}

export interface TrashResult {
  batchId: string;
  folders: number;
  notes: number;
}

export interface RestoreResult {
  folders: number;
  notes: number;
  reparented: number;
  renamed: number;
}

export interface ExportReport {
  path: string;
  notes: number;
  folders: number;
  attachments: number;
}

export interface ImportReport {
  foldersCreated: number;
  notesCreated: number;
  notesSkipped: number;
  notesTrashed: number;
  versionsImported: number;
  tagsAttached: number;
  attachmentsImported: number;
  attachmentsMissing: number;
}

export interface OrphanReport {
  /** Nombres de archivo borrados de `attachments/`. */
  removed: string[];
}

// ---------- Comandos ----------

export type CssKind = "theme" | "snippet";

export type ColorScheme = "system" | "light" | "dark";

export interface AppearanceSettings {
  /** `builtin:<id>`, `user:<nombre>` o null para el predeterminado. */
  theme: string | null;
  colorScheme: ColorScheme;
  snippets: string[];
}

export interface AppearanceState {
  themes: string[];
  snippets: string[];
  settings: AppearanceSettings;
}

export const api = {
  // App
  getAppInfo: () => invoke<AppInfo>("get_app_info"),

  // Apariencia
  getAppearance: () => invoke<AppearanceState>("get_appearance"),
  readAppearanceCss: (kind: CssKind, name: string) => invoke<string>("read_appearance_css", { kind, name }),
  setAppearance: (settings: AppearanceSettings) => invoke<void>("set_appearance", { settings }),
  createUserTheme: (name: string, css: string) => invoke<string>("create_user_theme", { name, css }),
  openAppearanceFolder: (kind: CssKind) => invoke<void>("open_appearance_folder", { kind }),

  // Carpetas
  listFolders: () => invoke<Folder[]>("list_folders"),
  folderTree: () => invoke<FolderNode[]>("folder_tree"),
  createFolder: (name: string, parentId: string | null = null) => invoke<Folder>("create_folder", { name, parentId }),
  renameFolder: (id: string, name: string) => invoke<Folder>("rename_folder", { id, name }),
  moveFolder: (id: string, targetParentId: string | null, expectedVersion: number) => invoke<Folder>("move_folder", { id, targetParentId, expectedVersion }),
  trashFolder: (id: string) => invoke<TrashResult>("trash_folder", { id }),

  // Notas
  listNotes: (tagId: string | null = null) => invoke<NoteSummary[]>("list_notes", { tagId }),
  getNote: (id: string) => invoke<Note>("get_note", { id }),
  createNote: (input: CreateNote) => invoke<Note>("create_note", { input }),
  updateNote: (id: string, input: UpdateNote) => invoke<Note>("update_note", { id, input }),
  moveNote: (id: string, folderId: string | null) => invoke<Note>("move_note", { id, folderId }),
  trashNote: (id: string) => invoke<void>("trash_note", { id }),
  listNoteVersions: (id: string) => invoke<NoteVersionSummary[]>("list_note_versions", { id }),
  getNoteVersion: (id: string, versionId: string) => invoke<NoteVersion>("get_note_version", { id, versionId }),
  restoreNoteVersion: (id: string, versionId: string) => invoke<Note>("restore_note_version", { id, versionId }),

  // Enlaces
  resolveLink: (title: string) => invoke<NoteSummary | null>("resolve_link", { title }),
  outgoingLinks: (id: string) => invoke<OutgoingLink[]>("outgoing_links", { id }),
  backlinks: (id: string) => invoke<NoteSummary[]>("backlinks", { id }),

  // Etiquetas
  listTags: () => invoke<Tag[]>("list_tags"),
  createTag: (name: string, color: string | null = null) => invoke<Tag>("create_tag", { name, color }),
  deleteTag: (id: string) => invoke<void>("delete_tag", { id }),
  noteTags: (id: string) => invoke<Tag[]>("note_tags", { id }),
  setNoteTags: (id: string, names: string[]) => invoke<Tag[]>("set_note_tags", { id, names }),

  // Búsqueda
  searchNotes: (query: string, limit?: number) => invoke<SearchHit[]>("search_notes", { query, limit }),

  // Papelera
  listTrash: () => invoke<TrashContents>("list_trash"),
  restoreNote: (id: string) => invoke<void>("restore_note", { id }),
  purgeNote: (id: string) => invoke<void>("purge_note", { id }),
  restoreTrashBatch: (batchId: string) => invoke<RestoreResult>("restore_trash_batch", { batchId }),
  purgeTrashBatch: (batchId: string) => invoke<void>("purge_trash_batch", { batchId }),
  emptyTrash: () => invoke<void>("empty_trash"),

  // Adjuntos: cuerpo binario crudo, sin JSON. Devuelve el nombre del archivo.
  saveAttachment: (bytes: Uint8Array) => invoke<string>("save_attachment", bytes),

  // Vault
  exportVault: (destPath: string, includeTrash = false) => invoke<ExportReport>("export_vault", { destPath, includeTrash }),
  importVault: (srcPath: string) => invoke<ImportReport>("import_vault", { srcPath }),
  importCogniVaultDb: (dbPath: string, uploadsDir: string | null = null) => invoke<ImportReport>("import_cogni_vault_db", { dbPath, uploadsDir }),
  pruneOrphanAttachments: () => invoke<OrphanReport>("prune_orphan_attachments"),
};

// ---------- Helpers ----------

/** Referencia que se escribe en el Markdown de la nota para un adjunto. */
export const attachmentRef = (name: string) => `attachments/${name}`;

/** URL servible por el protocolo `attachment://` (acepta `nombre` o `attachments/nombre`). */
export function attachmentUrl(nameOrRef: string): string {
  return convertFileSrc(nameOrRef.replace(/^attachments\//, ""), "attachment");
}

const HTML_ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** Convierte un snippet de búsqueda en HTML seguro: escapa y marca coincidencias con `<mark>`. */
export function highlight(snippet: string): string {
  return snippet
    .replace(/[&<>"']/g, (c) => HTML_ESCAPES[c])
    .replace(//g, "<mark>")
    .replace(//g, "</mark>");
}
