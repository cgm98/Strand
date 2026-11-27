use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub sort_order: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    #[serde(flatten)]
    pub folder: Folder,
    pub note_count: i64,
    pub children: Vec<FolderNode>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub body: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNote {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub folder_id: Option<String>,
}

/// Edición con control de concurrencia optimista: `expected_version` debe
/// coincidir con la versión guardada o se devuelve `CONFLICT`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNote {
    pub title: Option<String>,
    pub body: Option<String>,
    pub expected_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersionSummary {
    pub id: String,
    pub note_id: String,
    pub title: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: String,
    pub note_id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub note_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub updated_at: String,
    /// Fragmento con las coincidencias entre U+E000 y U+E001.
    pub snippet: String,
    pub rank: f64,
}

/// Un `[[enlace]]` saliente. `note_id` es `None` si aún no existe la nota.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingLink {
    pub title: String,
    pub note_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashBatch {
    pub id: String,
    pub root_folder_id: String,
    pub root_name: String,
    pub item_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashContents {
    /// Notas enviadas a la papelera una a una.
    pub notes: Vec<NoteSummary>,
    /// Carpetas enviadas a la papelera con todo su contenido.
    pub batches: Vec<TrashBatch>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashResult {
    pub batch_id: String,
    pub folders: usize,
    pub notes: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub folders: usize,
    pub notes: usize,
    /// Carpetas devueltas a la raíz porque su padre ya no existe.
    pub reparented: usize,
    /// Carpetas renombradas porque ya existía otra con el mismo nombre.
    pub renamed: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub path: String,
    pub notes: usize,
    pub folders: usize,
    pub attachments: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanReport {
    /// Nombres de archivo borrados de `attachments/`, ordenados.
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub folders_created: usize,
    pub notes_created: usize,
    pub notes_skipped: usize,
    pub notes_trashed: usize,
    pub versions_imported: usize,
    pub tags_attached: usize,
    pub attachments_imported: usize,
    pub attachments_missing: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub db_path: String,
}
