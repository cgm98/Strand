use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{
    CreateNote, Note, NoteSummary, NoteVersion, NoteVersionSummary, OutgoingLink, UpdateNote,
};
use crate::repo::folders;
use crate::util::{extract_wikilinks, new_id, now, title_key, validate_title};

/// Como mucho una instantánea automática por nota en esta ventana: el
/// autoguardado frecuente no llena el historial, pero siempre queda cómo
/// estaba la nota antes de cada sesión de edición.
pub const SNAPSHOT_WINDOW: Duration = Duration::minutes(15);

const COLS: &str = "id, folder_id, title, body, version, created_at, updated_at, deleted_at";
const SUMMARY_COLS: &str = "id, folder_id, title, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        version: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        deleted_at: r.get(7)?,
    })
}

pub(crate) fn summary_from_row(r: &Row) -> rusqlite::Result<NoteSummary> {
    Ok(NoteSummary {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        title: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
    })
}

/// Nota por id, incluida la papelera.
pub fn find(conn: &Connection, id: &str) -> AppResult<Option<Note>> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM notes WHERE id = ?1"), [id], from_row)
        .optional()?)
}

/// Nota por id (también en papelera, para poder previsualizarla).
pub fn get(conn: &Connection, id: &str) -> AppResult<Note> {
    find(conn, id)?.ok_or_else(|| AppError::not_found("Nota no encontrada"))
}

fn get_live(conn: &Connection, id: &str) -> AppResult<Note> {
    match find(conn, id)? {
        Some(n) if n.deleted_at.is_none() => Ok(n),
        _ => Err(AppError::not_found("Nota no encontrada")),
    }
}

/// Listado ligero (sin cuerpo) de todas las notas vivas, más recientes primero.
pub fn list(conn: &Connection) -> AppResult<Vec<NoteSummary>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {SUMMARY_COLS} FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC"
    ))?;
    let rows = stmt.query_map([], summary_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn list_by_tag(conn: &Connection, tag_id: &str) -> AppResult<Vec<NoteSummary>> {
    let mut stmt = conn.prepare_cached(
        "SELECT n.id, n.folder_id, n.title, n.created_at, n.updated_at
         FROM notes n JOIN note_tags nt ON nt.note_id = n.id
         WHERE nt.tag_id = ?1 AND n.deleted_at IS NULL ORDER BY n.updated_at DESC",
    )?;
    let rows = stmt.query_map([tag_id], summary_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, input: &CreateNote) -> AppResult<Note> {
    let title = validate_title(&input.title)?;
    folders::ensure_live(conn, input.folder_id.as_deref())?;
    let ts = now();
    let note = Note {
        id: new_id(),
        folder_id: input.folder_id.clone(),
        title,
        body: input.body.clone(),
        version: 1,
        created_at: ts.clone(),
        updated_at: ts,
        deleted_at: None,
    };
    insert_raw(conn, &note)?;
    Ok(note)
}

/// Inserta una nota con id y fechas dados (importadores). No valida carpeta.
pub(crate) fn insert_raw(conn: &Connection, n: &Note) -> AppResult<()> {
    conn.execute(
        "INSERT INTO notes (id, folder_id, title, title_key, body, version, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            n.id,
            n.folder_id,
            n.title,
            title_key(&n.title),
            n.body,
            n.version,
            n.created_at,
            n.updated_at,
            n.deleted_at
        ],
    )?;
    sync_links(conn, &n.id, &n.body)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &UpdateNote) -> AppResult<Note> {
    let tx = conn.unchecked_transaction()?;
    let existing = get_live(&tx, id)?;
    if existing.version != input.expected_version {
        return Err(AppError::conflict(format!(
            "La nota cambió en otra ventana (versión esperada {}, actual {}). Recárgala antes de guardar.",
            input.expected_version, existing.version
        )));
    }
    let title = match &input.title {
        Some(t) => validate_title(t)?,
        None => existing.title.clone(),
    };
    let body = input.body.clone().unwrap_or_else(|| existing.body.clone());
    if title == existing.title && body == existing.body {
        return Ok(existing);
    }

    let ts = now();
    if snapshot_due(&tx, id)? {
        insert_version(&tx, &existing, &ts)?;
    }
    tx.execute(
        "UPDATE notes SET title = ?2, title_key = ?3, body = ?4, version = version + 1, updated_at = ?5
         WHERE id = ?1",
        params![id, title, title_key(&title), body, ts],
    )?;
    if body != existing.body {
        sync_links(&tx, id, &body)?;
    }
    tx.commit()?;
    get(conn, id)
}

pub fn move_to(conn: &Connection, id: &str, folder_id: Option<&str>) -> AppResult<Note> {
    get_live(conn, id)?;
    folders::ensure_live(conn, folder_id)?;
    conn.execute(
        "UPDATE notes SET folder_id = ?2, version = version + 1, updated_at = ?3 WHERE id = ?1",
        params![id, folder_id, now()],
    )?;
    get(conn, id)
}

// ---------- Historial de versiones ----------

fn snapshot_due(conn: &Connection, note_id: &str) -> AppResult<bool> {
    let last: Option<String> = conn
        .query_row(
            "SELECT MAX(created_at) FROM note_versions WHERE note_id = ?1",
            [note_id],
            |r| r.get(0),
        )?;
    Ok(match last.and_then(|s| DateTime::parse_from_rfc3339(&s).ok()) {
        None => true,
        Some(t) => Utc::now().signed_duration_since(t) >= SNAPSHOT_WINDOW,
    })
}

fn insert_version(conn: &Connection, note: &Note, ts: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO note_versions (id, note_id, title, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_id(), note.id, note.title, note.body, ts],
    )?;
    Ok(())
}

pub fn list_versions(conn: &Connection, note_id: &str) -> AppResult<Vec<NoteVersionSummary>> {
    get(conn, note_id)?;
    let mut stmt = conn.prepare_cached(
        "SELECT id, note_id, title, created_at FROM note_versions
         WHERE note_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt
        .query_map([note_id], |r| {
            Ok(NoteVersionSummary {
                id: r.get(0)?,
                note_id: r.get(1)?,
                title: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_version(conn: &Connection, note_id: &str, version_id: &str) -> AppResult<NoteVersion> {
    conn.query_row(
        "SELECT id, note_id, title, body, created_at FROM note_versions WHERE id = ?1 AND note_id = ?2",
        params![version_id, note_id],
        |r| {
            Ok(NoteVersion {
                id: r.get(0)?,
                note_id: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                created_at: r.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("Versión no encontrada"))
}

/// Restaura una versión antigua. Antes guarda siempre el estado actual
/// (ignorando la ventana), así restaurar nunca pierde nada.
pub fn restore_version(conn: &Connection, note_id: &str, version_id: &str) -> AppResult<Note> {
    let tx = conn.unchecked_transaction()?;
    let current = get_live(&tx, note_id)?;
    let version = get_version(&tx, note_id, version_id)?;
    let ts = now();
    insert_version(&tx, &current, &ts)?;
    tx.execute(
        "UPDATE notes SET title = ?2, title_key = ?3, body = ?4, version = version + 1, updated_at = ?5
         WHERE id = ?1",
        params![note_id, version.title, title_key(&version.title), version.body, ts],
    )?;
    sync_links(&tx, note_id, &version.body)?;
    tx.commit()?;
    get(conn, note_id)
}

// ---------- Enlaces [[...]] ----------

pub(crate) fn sync_links(conn: &Connection, note_id: &str, body: &str) -> AppResult<()> {
    conn.execute("DELETE FROM note_links WHERE source_id = ?1", [note_id])?;
    let mut stmt = conn.prepare_cached(
        "INSERT INTO note_links (source_id, target_key, target_title) VALUES (?1, ?2, ?3)",
    )?;
    for (key, title) in extract_wikilinks(body) {
        stmt.execute(params![note_id, key, title])?;
    }
    Ok(())
}

/// Resuelve un título de enlace a la nota viva correspondiente (la más
/// reciente si hay varias con la misma clave).
pub fn resolve_title(conn: &Connection, title: &str) -> AppResult<Option<NoteSummary>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {SUMMARY_COLS} FROM notes WHERE title_key = ?1 AND deleted_at IS NULL
                 ORDER BY updated_at DESC LIMIT 1"
            ),
            [title_key(title)],
            summary_from_row,
        )
        .optional()?)
}

pub fn outgoing_links(conn: &Connection, note_id: &str) -> AppResult<Vec<OutgoingLink>> {
    let mut stmt = conn.prepare_cached(
        "SELECT l.target_title,
                (SELECT n.id FROM notes n WHERE n.title_key = l.target_key AND n.deleted_at IS NULL
                 ORDER BY n.updated_at DESC LIMIT 1)
         FROM note_links l WHERE l.source_id = ?1 ORDER BY l.target_title COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([note_id], |r| Ok(OutgoingLink { title: r.get(0)?, note_id: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn backlinks(conn: &Connection, note_id: &str) -> AppResult<Vec<NoteSummary>> {
    let note = get(conn, note_id)?;
    let mut stmt = conn.prepare_cached(
        "SELECT n.id, n.folder_id, n.title, n.created_at, n.updated_at
         FROM note_links l JOIN notes n ON n.id = l.source_id
         WHERE l.target_key = ?1 AND n.deleted_at IS NULL AND n.id <> ?2
         ORDER BY n.title COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map(params![title_key(&note.title), note_id], summary_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn new(conn: &Connection, title: &str, body: &str) -> Note {
        create(conn, &CreateNote { title: title.into(), body: body.into(), folder_id: None }).unwrap()
    }

    #[test]
    fn update_requires_matching_version() {
        let conn = open_in_memory().unwrap();
        let n = new(&conn, "Hola", "uno");
        let up = UpdateNote { title: None, body: Some("dos".into()), expected_version: n.version };
        let n2 = update(&conn, &n.id, &up).unwrap();
        assert_eq!(n2.version, 2);
        // Reintentar con la versión vieja = otra ventana con datos obsoletos.
        assert_eq!(update(&conn, &n.id, &up).unwrap_err().code(), "CONFLICT");
    }

    #[test]
    fn snapshots_once_per_window_and_restore_keeps_current() {
        let conn = open_in_memory().unwrap();
        let mut n = new(&conn, "Nota", "v1");
        for body in ["v2", "v3", "v4"] {
            n = update(&conn, &n.id, &UpdateNote { title: None, body: Some(body.into()), expected_version: n.version }).unwrap();
        }
        let versions = list_versions(&conn, &n.id).unwrap();
        assert_eq!(versions.len(), 1, "una sola instantánea dentro de la ventana");
        let v1 = get_version(&conn, &n.id, &versions[0].id).unwrap();
        assert_eq!(v1.body, "v1");

        let restored = restore_version(&conn, &n.id, &v1.id).unwrap();
        assert_eq!(restored.body, "v1");
        let after = list_versions(&conn, &n.id).unwrap();
        assert_eq!(after.len(), 2);
        assert!(after.iter().any(|v| get_version(&conn, &n.id, &v.id).unwrap().body == "v4"));
    }

    #[test]
    fn links_and_backlinks_resolve_by_title_key() {
        let conn = open_in_memory().unwrap();
        let a = new(&conn, "Nota A", "Ver [[Domain Core]] y [[Pendiente]]");
        let b = new(&conn, "DomainCore", "");
        let out = outgoing_links(&conn, &a.id).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out.iter().find(|l| l.title == "Domain Core").unwrap().note_id.as_deref(), Some(b.id.as_str()));
        assert!(out.iter().find(|l| l.title == "Pendiente").unwrap().note_id.is_none());
        let back = backlinks(&conn, &b.id).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, a.id);

        // Crear la nota que faltaba conecta el enlace sin tocar A.
        let p = new(&conn, "pendiente", "");
        assert_eq!(backlinks(&conn, &p.id).unwrap()[0].id, a.id);
    }

    #[test]
    fn create_rejects_missing_folder_and_empty_title() {
        let conn = open_in_memory().unwrap();
        let bad = CreateNote { title: "x".into(), body: String::new(), folder_id: Some("nope".into()) };
        assert_eq!(create(&conn, &bad).unwrap_err().code(), "VALIDATION");
        let empty = CreateNote { title: "  ".into(), body: String::new(), folder_id: None };
        assert_eq!(create(&conn, &empty).unwrap_err().code(), "VALIDATION");
    }
}
