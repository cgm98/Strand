//! Capa IPC: comandos delgados que validan entrada, toman la conexión y
//! delegan en `repo`/`vault`. Toda la lógica vive fuera de aquí.
//!
//! Los comandos son `async` para no bloquear el hilo principal (la UI); las
//! operaciones largas (importar/exportar) corren además en `spawn_blocking`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use tauri::{AppHandle, State};

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::repo::{folders, notes, search, tags, trash};
use crate::{appearance, attachments, vault};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub attachments_dir: PathBuf,
}

impl AppState {
    fn conn(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.db.lock().map_err(|_| AppError::Internal("La conexión a la base de datos quedó inconsistente".into()))
    }
}

/// Ejecuta trabajo pesado de BD fuera del runtime async.
async fn blocking<T, F>(state: &AppState, f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Connection, &std::path::Path) -> AppResult<T> + Send + 'static,
{
    let db = Arc::clone(&state.db);
    let att = state.attachments_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| AppError::Internal("Conexión inconsistente".into()))?;
        f(&conn, &att)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

// ---------- App ----------

#[tauri::command]
pub async fn get_app_info(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppInfo> {
    let pkg = app.package_info();
    Ok(AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        data_dir: state.data_dir.display().to_string(),
        db_path: state.db_path.display().to_string(),
    })
}

// ---------- Carpetas ----------

#[tauri::command]
pub async fn list_folders(state: State<'_, AppState>) -> AppResult<Vec<Folder>> {
    folders::list(&*state.conn()?)
}

#[tauri::command]
pub async fn folder_tree(state: State<'_, AppState>) -> AppResult<Vec<FolderNode>> {
    folders::tree(&*state.conn()?)
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, name: String, parent_id: Option<String>) -> AppResult<Folder> {
    folders::create(&*state.conn()?, &name, parent_id.as_deref())
}

#[tauri::command]
pub async fn rename_folder(state: State<'_, AppState>, id: String, name: String) -> AppResult<Folder> {
    folders::rename(&*state.conn()?, &id, &name)
}

#[tauri::command]
pub async fn move_folder(
    state: State<'_, AppState>,
    id: String,
    target_parent_id: Option<String>,
    expected_version: i64,
) -> AppResult<Folder> {
    folders::move_to(&*state.conn()?, &id, target_parent_id.as_deref(), expected_version)
}

#[tauri::command]
pub async fn trash_folder(state: State<'_, AppState>, id: String) -> AppResult<TrashResult> {
    trash::trash_folder(&*state.conn()?, &id)
}

// ---------- Notas ----------

#[tauri::command]
pub async fn list_notes(state: State<'_, AppState>, tag_id: Option<String>) -> AppResult<Vec<NoteSummary>> {
    let conn = state.conn()?;
    match tag_id {
        Some(t) => notes::list_by_tag(&conn, &t),
        None => notes::list(&conn),
    }
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> AppResult<Note> {
    notes::get(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn create_note(state: State<'_, AppState>, input: CreateNote) -> AppResult<Note> {
    notes::create(&*state.conn()?, &input)
}

#[tauri::command]
pub async fn update_note(state: State<'_, AppState>, id: String, input: UpdateNote) -> AppResult<Note> {
    notes::update(&*state.conn()?, &id, &input)
}

#[tauri::command]
pub async fn move_note(state: State<'_, AppState>, id: String, folder_id: Option<String>) -> AppResult<Note> {
    notes::move_to(&*state.conn()?, &id, folder_id.as_deref())
}

#[tauri::command]
pub async fn trash_note(state: State<'_, AppState>, id: String) -> AppResult<()> {
    trash::trash_note(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn list_note_versions(state: State<'_, AppState>, id: String) -> AppResult<Vec<NoteVersionSummary>> {
    notes::list_versions(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn get_note_version(state: State<'_, AppState>, id: String, version_id: String) -> AppResult<NoteVersion> {
    notes::get_version(&*state.conn()?, &id, &version_id)
}

#[tauri::command]
pub async fn restore_note_version(state: State<'_, AppState>, id: String, version_id: String) -> AppResult<Note> {
    notes::restore_version(&*state.conn()?, &id, &version_id)
}

// ---------- Enlaces ----------

#[tauri::command]
pub async fn resolve_link(state: State<'_, AppState>, title: String) -> AppResult<Option<NoteSummary>> {
    notes::resolve_title(&*state.conn()?, &title)
}

#[tauri::command]
pub async fn outgoing_links(state: State<'_, AppState>, id: String) -> AppResult<Vec<OutgoingLink>> {
    notes::outgoing_links(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn backlinks(state: State<'_, AppState>, id: String) -> AppResult<Vec<NoteSummary>> {
    notes::backlinks(&*state.conn()?, &id)
}

// ---------- Etiquetas ----------

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> AppResult<Vec<Tag>> {
    tags::list(&*state.conn()?)
}

#[tauri::command]
pub async fn create_tag(state: State<'_, AppState>, name: String, color: Option<String>) -> AppResult<Tag> {
    tags::create(&*state.conn()?, &name, color.as_deref())
}

#[tauri::command]
pub async fn delete_tag(state: State<'_, AppState>, id: String) -> AppResult<()> {
    tags::delete(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn note_tags(state: State<'_, AppState>, id: String) -> AppResult<Vec<Tag>> {
    tags::for_note(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn set_note_tags(state: State<'_, AppState>, id: String, names: Vec<String>) -> AppResult<Vec<Tag>> {
    tags::set_for_note(&*state.conn()?, &id, &names)
}

// ---------- Búsqueda ----------

#[tauri::command]
pub async fn search_notes(state: State<'_, AppState>, query: String, limit: Option<u32>) -> AppResult<Vec<SearchHit>> {
    search::search(&*state.conn()?, &query, limit.unwrap_or(50))
}

// ---------- Papelera ----------

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>) -> AppResult<TrashContents> {
    trash::list(&*state.conn()?)
}

#[tauri::command]
pub async fn restore_note(state: State<'_, AppState>, id: String) -> AppResult<()> {
    trash::restore_note(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn purge_note(state: State<'_, AppState>, id: String) -> AppResult<()> {
    trash::purge_note(&*state.conn()?, &id)
}

#[tauri::command]
pub async fn restore_trash_batch(state: State<'_, AppState>, batch_id: String) -> AppResult<RestoreResult> {
    trash::restore_batch(&*state.conn()?, &batch_id)
}

#[tauri::command]
pub async fn purge_trash_batch(state: State<'_, AppState>, batch_id: String) -> AppResult<()> {
    trash::purge_batch(&*state.conn()?, &batch_id)
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> AppResult<()> {
    trash::empty(&*state.conn()?)
}

// ---------- Adjuntos ----------

/// Cuerpo binario crudo (Uint8Array desde el frontend), sin pasar por JSON.
/// Devuelve el nombre; en la nota se referencia como `attachments/<nombre>`.
#[tauri::command]
pub async fn save_attachment(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> AppResult<String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::validation("Se esperaba el contenido binario de la imagen"));
    };
    attachments::save(&state.attachments_dir, bytes)
}

/// Borra los archivos de `attachments/` que ninguna nota (viva, en la
/// papelera) ni versión referencia. Irreversible: el frontend debe confirmar
/// antes de llamarlo.
#[tauri::command]
pub async fn prune_orphan_attachments(state: State<'_, AppState>) -> AppResult<OrphanReport> {
    blocking(&state, |conn, att| Ok(OrphanReport { removed: attachments::prune_orphans(conn, att)? })).await
}

// ---------- Vault ----------

#[tauri::command]
pub async fn export_vault(state: State<'_, AppState>, dest_path: String, include_trash: bool) -> AppResult<ExportReport> {
    let dest = PathBuf::from(dest_path);
    blocking(&state, move |conn, att| vault::export::export_zip(conn, att, &dest, include_trash)).await
}

#[tauri::command]
pub async fn import_vault(state: State<'_, AppState>, src_path: String) -> AppResult<ImportReport> {
    let src = PathBuf::from(src_path);
    blocking(&state, move |conn, att| vault::import_zip::import_zip(conn, att, &src)).await
}

/// Migra una base `cogni_vault.db`. `uploads_dir` es opcional: si no se da,
/// se buscan las imágenes junto a la base (`uploads/`, `../uploads/`,
/// `../backend/uploads/`).
#[tauri::command]
pub async fn import_cogni_vault_db(
    state: State<'_, AppState>,
    db_path: String,
    uploads_dir: Option<String>,
) -> AppResult<ImportReport> {
    let src = PathBuf::from(db_path);
    let uploads = uploads_dir.map(PathBuf::from);
    blocking(&state, move |conn, att| {
        vault::import_legacy::import_legacy_db(conn, att, &src, uploads.as_deref())
    })
    .await
}

// ---------- Apariencia (temas y fragmentos CSS) ----------

#[tauri::command]
pub async fn get_appearance(state: State<'_, AppState>) -> AppResult<appearance::AppearanceState> {
    appearance::ensure_dirs(&state.data_dir)?;
    appearance::state(&state.data_dir)
}

#[tauri::command]
pub async fn read_appearance_css(state: State<'_, AppState>, kind: appearance::CssKind, name: String) -> AppResult<String> {
    appearance::read(&state.data_dir, kind, &name)
}

#[tauri::command]
pub async fn set_appearance(state: State<'_, AppState>, settings: appearance::AppearanceSettings) -> AppResult<()> {
    appearance::save_settings(&state.data_dir, &settings)
}

/// Copia un tema (normalmente uno incluido) a la carpeta del usuario para editarlo.
#[tauri::command]
pub async fn create_user_theme(state: State<'_, AppState>, name: String, css: String) -> AppResult<String> {
    appearance::create_theme(&state.data_dir, &name, &css)
}

/// Abre la carpeta en el gestor de archivos para que el usuario añada o edite CSS.
#[tauri::command]
pub async fn open_appearance_folder(app: AppHandle, state: State<'_, AppState>, kind: appearance::CssKind) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    appearance::ensure_dirs(&state.data_dir)?;
    let dir = kind.dir(&state.data_dir);
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Internal(format!("No se pudo abrir la carpeta: {e}")))
}
