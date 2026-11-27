mod appearance;
mod attachments;
mod commands;
mod db;
mod error;
mod models;
mod repo;
mod util;
mod vault;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use commands::AppState;

/// Esquema del protocolo de imágenes: `attachment://localhost/<nombre>`.
pub const ATTACHMENT_SCHEME: &str = "attachment";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // ~/.local/share/io.github.cgm98.strand (o la ruta del sandbox en Flatpak)
            let data_dir = app.path().app_data_dir()?;
            let attachments_dir = data_dir.join("attachments");
            std::fs::create_dir_all(&attachments_dir)?;
            let db_path = data_dir.join("notas.db");
            let conn = db::open(&db_path)?;
            app.manage(AppState { db: Arc::new(Mutex::new(conn)), data_dir, db_path, attachments_dir });
            Ok(())
        })
        .register_uri_scheme_protocol(ATTACHMENT_SCHEME, |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            attachments::serve(&state.attachments_dir, request.uri().path())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_folders,
            commands::folder_tree,
            commands::create_folder,
            commands::rename_folder,
            commands::move_folder,
            commands::trash_folder,
            commands::list_notes,
            commands::get_note,
            commands::create_note,
            commands::update_note,
            commands::move_note,
            commands::trash_note,
            commands::list_note_versions,
            commands::get_note_version,
            commands::restore_note_version,
            commands::resolve_link,
            commands::outgoing_links,
            commands::backlinks,
            commands::list_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::note_tags,
            commands::set_note_tags,
            commands::search_notes,
            commands::list_trash,
            commands::restore_note,
            commands::purge_note,
            commands::restore_trash_batch,
            commands::purge_trash_batch,
            commands::empty_trash,
            commands::save_attachment,
            commands::prune_orphan_attachments,
            commands::export_vault,
            commands::import_vault,
            commands::import_cogni_vault_db,
            commands::get_appearance,
            commands::read_appearance_css,
            commands::set_appearance,
            commands::create_user_theme,
            commands::open_appearance_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
