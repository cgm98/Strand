use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{is_unique_violation, AppError, AppResult};
use crate::models::Tag;
use crate::util::{new_id, now};

/// Las etiquetas se guardan normalizadas: sin espacios extremos, sin `#`
/// inicial y en minúsculas.
pub fn normalize(name: &str) -> AppResult<String> {
    let n = name.trim().trim_start_matches('#').trim().to_lowercase();
    if n.is_empty() {
        return Err(AppError::validation("La etiqueta no puede estar vacía"));
    }
    if n.chars().count() > 50 {
        return Err(AppError::validation("La etiqueta no puede superar 50 caracteres"));
    }
    Ok(n)
}

pub fn list(conn: &Connection) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.id, t.name, t.color,
                (SELECT COUNT(*) FROM note_tags nt JOIN notes n ON n.id = nt.note_id
                 WHERE nt.tag_id = t.id AND n.deleted_at IS NULL)
         FROM tags t ORDER BY t.name",
    )?;
    let rows = stmt
        .query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, note_count: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, name: &str, color: Option<&str>) -> AppResult<Tag> {
    let name = normalize(name)?;
    let id = new_id();
    conn.execute(
        "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, now()],
    )
    .map_err(|e| {
        if is_unique_violation(&e) {
            AppError::conflict(format!("La etiqueta \"{name}\" ya existe"))
        } else {
            e.into()
        }
    })?;
    Ok(Tag { id, name, color: color.map(str::to_string), note_count: 0 })
}

pub fn find_or_create(conn: &Connection, name: &str) -> AppResult<String> {
    let name = normalize(name)?;
    if let Some(id) = conn
        .query_row("SELECT id FROM tags WHERE name = ?1", [&name], |r| r.get(0))
        .optional()?
    {
        return Ok(id);
    }
    Ok(create(conn, &name, None)?.id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    if conn.execute("DELETE FROM tags WHERE id = ?1", [id])? == 0 {
        return Err(AppError::not_found("Etiqueta no encontrada"));
    }
    Ok(())
}

pub fn for_note(conn: &Connection, note_id: &str) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare_cached(
        "SELECT t.id, t.name, t.color, 0 FROM tags t JOIN note_tags nt ON nt.tag_id = t.id
         WHERE nt.note_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt
        .query_map([note_id], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, note_count: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn attach(conn: &Connection, note_id: &str, tag_id: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO note_tags (note_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
        params![note_id, tag_id, now()],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(ref f, _)
            if f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
        {
            AppError::not_found("Nota o etiqueta no encontrada")
        }
        e => e.into(),
    })?;
    Ok(())
}

/// Reemplaza las etiquetas de una nota por `names`, creando las que falten.
pub fn set_for_note(conn: &Connection, note_id: &str, names: &[String]) -> AppResult<Vec<Tag>> {
    crate::repo::notes::get(conn, note_id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM note_tags WHERE note_id = ?1", [note_id])?;
    for name in names {
        let tag_id = find_or_create(&tx, name)?;
        attach(&tx, note_id, &tag_id)?;
    }
    tx.commit()?;
    for_note(conn, note_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::models::CreateNote;

    #[test]
    fn set_for_note_normalizes_and_dedupes() {
        let conn = open_in_memory().unwrap();
        let n = crate::repo::notes::create(&conn, &CreateNote { title: "t".into(), body: String::new(), folder_id: None }).unwrap();
        let tags = set_for_note(&conn, &n.id, &["#Rust".into(), "rust".into(), " Tauri ".into()]).unwrap();
        let names: Vec<_> = tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["rust", "tauri"]);
        assert_eq!(list(&conn).unwrap().iter().find(|t| t.name == "rust").unwrap().note_count, 1);
        assert_eq!(create(&conn, "RUST", None).unwrap_err().code(), "CONFLICT");
    }
}
