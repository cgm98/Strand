//! Papelera. Nada se borra de verdad hasta un `purge` explícito.
//!
//! - Una nota suelta va a la papelera sola (`trash_batch_id` NULL).
//! - Una carpeta va con todo su subárbol vivo en un lote (`trash_batches`),
//!   que se restaura o purga de una vez.

use rusqlite::{params, Connection};

use crate::error::{AppError, AppResult};
use crate::models::{RestoreResult, TrashBatch, TrashContents, TrashResult};
use crate::repo::{folders, notes};
use crate::util::{new_id, now};

pub fn trash_note(conn: &Connection, id: &str) -> AppResult<()> {
    let note = notes::get(conn, id)?;
    if note.deleted_at.is_some() {
        return Ok(()); // idempotente
    }
    conn.execute(
        "UPDATE notes SET deleted_at = ?2, trash_batch_id = NULL WHERE id = ?1",
        params![id, now()],
    )?;
    Ok(())
}

/// Devuelve una nota a su carpeta, o a la raíz si su carpeta ya no está viva.
pub fn restore_note(conn: &Connection, id: &str) -> AppResult<()> {
    let note = notes::get(conn, id)?;
    if note.deleted_at.is_none() {
        return Err(AppError::not_found("La nota no está en la papelera"));
    }
    let folder_alive = match note.folder_id.as_deref() {
        Some(f) => folders::get(conn, f).is_ok(),
        None => true,
    };
    conn.execute(
        "UPDATE notes SET deleted_at = NULL, trash_batch_id = NULL,
                folder_id = CASE WHEN ?2 THEN folder_id ELSE NULL END
         WHERE id = ?1",
        params![id, folder_alive],
    )?;
    Ok(())
}

/// Borra definitivamente una nota que ya está en la papelera (con su
/// historial, etiquetas y enlaces por cascada).
pub fn purge_note(conn: &Connection, id: &str) -> AppResult<()> {
    let note = notes::get(conn, id)?;
    if note.deleted_at.is_none() {
        return Err(AppError::validation("Solo se pueden borrar definitivamente notas de la papelera"));
    }
    conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    Ok(())
}

pub fn trash_folder(conn: &Connection, id: &str) -> AppResult<TrashResult> {
    let root = folders::get(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    let batch_id = new_id();
    let ts = now();
    tx.execute(
        "INSERT INTO trash_batches (id, root_folder_id, root_name, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![batch_id, root.id, root.name, ts],
    )?;
    tx.execute_batch("CREATE TEMP TABLE IF NOT EXISTS _subtree (id TEXT PRIMARY KEY); DELETE FROM _subtree;")?;
    tx.execute(
        "INSERT INTO _subtree (id)
         WITH RECURSIVE sub(id) AS (
             SELECT id FROM folders WHERE id = ?1 AND deleted_at IS NULL
             UNION
             SELECT f.id FROM folders f JOIN sub ON f.parent_id = sub.id WHERE f.deleted_at IS NULL
         ) SELECT id FROM sub",
        [id],
    )?;
    let folders_n = tx.execute(
        "UPDATE folders SET deleted_at = ?1, trash_batch_id = ?2 WHERE id IN (SELECT id FROM _subtree)",
        params![ts, batch_id],
    )?;
    let notes_n = tx.execute(
        "UPDATE notes SET deleted_at = ?1, trash_batch_id = ?2
         WHERE deleted_at IS NULL AND folder_id IN (SELECT id FROM _subtree)",
        params![ts, batch_id],
    )?;
    tx.execute(
        "UPDATE trash_batches SET item_count = ?2 WHERE id = ?1",
        params![batch_id, (folders_n + notes_n) as i64],
    )?;
    tx.execute("DELETE FROM _subtree", [])?;
    tx.commit()?;
    Ok(TrashResult { batch_id, folders: folders_n, notes: notes_n })
}

fn pending_batch(conn: &Connection, batch_id: &str) -> AppResult<()> {
    let ok: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM trash_batches WHERE id = ?1 AND restored_at IS NULL AND purged_at IS NULL)",
        [batch_id],
        |r| r.get(0),
    )?;
    if ok { Ok(()) } else { Err(AppError::not_found("Lote de papelera no encontrado o ya procesado")) }
}

pub fn restore_batch(conn: &Connection, batch_id: &str) -> AppResult<RestoreResult> {
    pending_batch(conn, batch_id)?;
    let tx = conn.unchecked_transaction()?;
    let ts = now();
    let mut result = RestoreResult::default();

    // Padres antes que hijos, para que cada hijo vea a su padre ya restaurado.
    let batch_folders: Vec<(String, Option<String>, String)> = {
        let mut stmt = tx.prepare(
            "WITH RECURSIVE ordered(id, depth) AS (
                 SELECT id, 0 FROM folders f WHERE trash_batch_id = ?1
                   AND (parent_id IS NULL OR parent_id NOT IN (SELECT id FROM folders WHERE trash_batch_id = ?1))
                 UNION ALL
                 SELECT f.id, o.depth + 1 FROM folders f JOIN ordered o ON f.parent_id = o.id
                 WHERE f.trash_batch_id = ?1
             )
             SELECT f.id, f.parent_id, f.name FROM ordered o JOIN folders f ON f.id = o.id ORDER BY o.depth",
        )?;
        stmt.query_map([batch_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?
    };

    for (id, parent_id, name) in batch_folders {
        let parent = match parent_id {
            Some(p) if folders::get(&tx, &p).is_ok() => Some(p),
            Some(_) => {
                result.reparented += 1;
                None
            }
            None => None,
        };
        let free_name = folders::unique_name(&tx, parent.as_deref(), &name, &id)?;
        if free_name != name {
            result.renamed += 1;
        }
        tx.execute(
            "UPDATE folders SET deleted_at = NULL, trash_batch_id = NULL, parent_id = ?2, name = ?3,
                    version = version + 1, updated_at = ?4
             WHERE id = ?1",
            params![id, parent, free_name, ts],
        )?;
        result.folders += 1;
    }

    result.notes = tx.execute(
        "UPDATE notes SET deleted_at = NULL, trash_batch_id = NULL WHERE trash_batch_id = ?1",
        [batch_id],
    )?;
    tx.execute("UPDATE trash_batches SET restored_at = ?2 WHERE id = ?1", params![batch_id, ts])?;
    tx.commit()?;
    Ok(result)
}

pub fn purge_batch(conn: &Connection, batch_id: &str) -> AppResult<()> {
    pending_batch(conn, batch_id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM notes WHERE trash_batch_id = ?1", [batch_id])?;
    // Carpetas de otros lotes que colgaban de este: pasan a la raíz para
    // que el borrado no deje referencias rotas.
    tx.execute(
        "UPDATE folders SET parent_id = NULL
         WHERE ifnull(trash_batch_id, '') <> ?1
           AND parent_id IN (SELECT id FROM folders WHERE trash_batch_id = ?1)",
        [batch_id],
    )?;
    tx.execute("DELETE FROM folders WHERE trash_batch_id = ?1", [batch_id])?;
    tx.execute("UPDATE trash_batches SET purged_at = ?2 WHERE id = ?1", params![batch_id, now()])?;
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<TrashContents> {
    let notes = {
        let mut stmt = conn.prepare_cached(
            "SELECT id, folder_id, title, created_at, updated_at FROM notes
             WHERE deleted_at IS NOT NULL AND trash_batch_id IS NULL ORDER BY deleted_at DESC",
        )?;
        stmt.query_map([], notes::summary_from_row)?.collect::<Result<Vec<_>, _>>()?
    };
    let batches = {
        let mut stmt = conn.prepare_cached(
            "SELECT id, root_folder_id, root_name, item_count, created_at FROM trash_batches
             WHERE restored_at IS NULL AND purged_at IS NULL ORDER BY created_at DESC",
        )?;
        stmt.query_map([], |r| {
            Ok(TrashBatch {
                id: r.get(0)?,
                root_folder_id: r.get(1)?,
                root_name: r.get(2)?,
                item_count: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
    };
    Ok(TrashContents { notes, batches })
}

/// Vacía la papelera entera. Irreversible.
pub fn empty(conn: &Connection) -> AppResult<()> {
    let contents = list(conn)?;
    for b in &contents.batches {
        purge_batch(conn, &b.id)?;
    }
    conn.execute("DELETE FROM notes WHERE deleted_at IS NOT NULL", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::models::CreateNote;

    fn note_in(conn: &Connection, folder: Option<&str>, title: &str) -> String {
        notes::create(conn, &CreateNote { title: title.into(), body: String::new(), folder_id: folder.map(String::from) })
            .unwrap()
            .id
    }

    #[test]
    fn folder_subtree_roundtrip() {
        let conn = open_in_memory().unwrap();
        let a = folders::create(&conn, "A", None).unwrap();
        let b = folders::create(&conn, "B", Some(&a.id)).unwrap();
        let n1 = note_in(&conn, Some(&a.id), "n1");
        let n2 = note_in(&conn, Some(&b.id), "n2");

        let r = trash_folder(&conn, &a.id).unwrap();
        assert_eq!((r.folders, r.notes), (2, 2));
        assert!(folders::list(&conn).unwrap().is_empty());
        assert!(notes::list(&conn).unwrap().is_empty());
        assert_eq!(list(&conn).unwrap().batches.len(), 1);

        // Mientras tanto se crea otra "A": la restaurada se renombra.
        folders::create(&conn, "A", None).unwrap();
        let rr = restore_batch(&conn, &r.batch_id).unwrap();
        assert_eq!((rr.folders, rr.notes, rr.renamed), (2, 2, 1));
        assert_eq!(folders::get(&conn, &a.id).unwrap().name, "A (2)");
        assert_eq!(folders::get(&conn, &b.id).unwrap().parent_id.as_deref(), Some(a.id.as_str()));
        assert!(notes::get(&conn, &n1).unwrap().deleted_at.is_none());
        assert!(notes::get(&conn, &n2).unwrap().deleted_at.is_none());
        assert!(restore_batch(&conn, &r.batch_id).is_err(), "no se restaura dos veces");
    }

    #[test]
    fn nested_batches_purge_without_fk_errors() {
        let conn = open_in_memory().unwrap();
        let a = folders::create(&conn, "A", None).unwrap();
        let b = folders::create(&conn, "B", Some(&a.id)).unwrap();
        let nb = note_in(&conn, Some(&b.id), "nb");
        let inner = trash_folder(&conn, &b.id).unwrap();
        let outer = trash_folder(&conn, &a.id).unwrap();

        purge_batch(&conn, &outer.batch_id).unwrap();
        assert!(folders::find(&conn, &a.id).unwrap().is_none());

        // El lote interior sigue restaurable y su carpeta vuelve a la raíz.
        let rr = restore_batch(&conn, &inner.batch_id).unwrap();
        assert_eq!(rr.reparented, 0, "ya se había desenganchado al purgar");
        assert_eq!(folders::get(&conn, &b.id).unwrap().parent_id, None);
        assert!(notes::get(&conn, &nb).unwrap().deleted_at.is_none());
    }

    #[test]
    fn single_note_restore_falls_back_to_root() {
        let conn = open_in_memory().unwrap();
        let a = folders::create(&conn, "A", None).unwrap();
        let n = note_in(&conn, Some(&a.id), "n");
        trash_note(&conn, &n).unwrap();
        trash_folder(&conn, &a.id).unwrap();
        assert_eq!(list(&conn).unwrap().notes.len(), 1);
        restore_note(&conn, &n).unwrap();
        assert_eq!(notes::get(&conn, &n).unwrap().folder_id, None);
        assert_eq!(purge_note(&conn, &n).unwrap_err().code(), "VALIDATION");
        trash_note(&conn, &n).unwrap();
        empty(&conn).unwrap();
        assert!(notes::find(&conn, &n).unwrap().is_none());
        assert!(folders::find(&conn, &a.id).unwrap().is_none());
    }
}
