use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{is_unique_violation, AppError, AppResult};
use crate::models::{Folder, FolderNode};
use crate::util::{now, new_id, validate_folder_name};

const COLS: &str = "id, parent_id, name, sort_order, version, created_at, updated_at, deleted_at";

fn from_row(r: &Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        name: r.get(2)?,
        sort_order: r.get(3)?,
        version: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        deleted_at: r.get(7)?,
    })
}

fn duplicate_name(name: &str) -> AppError {
    AppError::conflict(format!("Ya existe una carpeta llamada \"{name}\" en esta ubicación"))
}

/// Carpeta por id, incluida la papelera.
pub fn find(conn: &Connection, id: &str) -> AppResult<Option<Folder>> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM folders WHERE id = ?1"), [id], from_row)
        .optional()?)
}

/// Carpeta viva por id; error si no existe o está en la papelera.
pub fn get(conn: &Connection, id: &str) -> AppResult<Folder> {
    match find(conn, id)? {
        Some(f) if f.deleted_at.is_none() => Ok(f),
        _ => Err(AppError::not_found("Carpeta no encontrada")),
    }
}

pub fn ensure_live(conn: &Connection, id: Option<&str>) -> AppResult<()> {
    if let Some(id) = id {
        get(conn, id).map_err(|_| AppError::validation("La carpeta de destino no existe"))?;
    }
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<Folder>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {COLS} FROM folders WHERE deleted_at IS NULL
         ORDER BY sort_order, name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn tree(conn: &Connection) -> AppResult<Vec<FolderNode>> {
    let folders = list(conn)?;
    let mut counts: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = conn.prepare_cached(
            "SELECT folder_id, COUNT(*) FROM notes
             WHERE deleted_at IS NULL AND folder_id IS NOT NULL GROUP BY folder_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (id, n) = row?;
            counts.insert(id, n);
        }
    }

    let mut children: HashMap<Option<String>, Vec<Folder>> = HashMap::new();
    let ids: std::collections::HashSet<String> = folders.iter().map(|f| f.id.clone()).collect();
    for f in folders {
        // Un padre inexistente no debería ocurrir; si ocurre, la carpeta
        // se muestra en la raíz en lugar de desaparecer.
        let parent = f.parent_id.clone().filter(|p| ids.contains(p));
        children.entry(parent).or_default().push(f);
    }

    fn build(
        parent: Option<String>,
        children: &mut HashMap<Option<String>, Vec<Folder>>,
        counts: &HashMap<String, i64>,
    ) -> Vec<FolderNode> {
        let Some(list) = children.remove(&parent) else { return Vec::new() };
        list.into_iter()
            .map(|folder| {
                let id = folder.id.clone();
                FolderNode {
                    note_count: counts.get(&id).copied().unwrap_or(0),
                    children: build(Some(id), children, counts),
                    folder,
                }
            })
            .collect()
    }

    Ok(build(None, &mut children, &counts))
}

pub fn create(conn: &Connection, name: &str, parent_id: Option<&str>) -> AppResult<Folder> {
    let name = validate_folder_name(name)?;
    if let Some(pid) = parent_id {
        get(conn, pid).map_err(|_| AppError::validation("La carpeta padre no existe"))?;
    }
    let id = new_id();
    let ts = now();
    conn.execute(
        "INSERT INTO folders (id, parent_id, name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, parent_id, name, ts],
    )
    .map_err(|e| if is_unique_violation(&e) { duplicate_name(&name) } else { e.into() })?;
    get(conn, &id)
}

/// Busca una carpeta viva por nombre (sin distinguir mayúsculas) bajo
/// `parent_id`, o la crea. Usado por los importadores.
pub fn find_or_create(conn: &Connection, name: &str, parent_id: Option<&str>) -> AppResult<(Folder, bool)> {
    let name = validate_folder_name(name)?;
    let existing = conn
        .query_row(
            &format!(
                "SELECT {COLS} FROM folders
                 WHERE deleted_at IS NULL AND ifnull(parent_id, '') = ifnull(?1, '')
                   AND name = ?2 COLLATE NOCASE"
            ),
            params![parent_id, name],
            from_row,
        )
        .optional()?;
    match existing {
        Some(f) => Ok((f, false)),
        None => Ok((create(conn, &name, parent_id)?, true)),
    }
}

pub fn rename(conn: &Connection, id: &str, name: &str) -> AppResult<Folder> {
    let name = validate_folder_name(name)?;
    get(conn, id)?;
    conn.execute(
        "UPDATE folders SET name = ?2, version = version + 1, updated_at = ?3 WHERE id = ?1",
        params![id, name, now()],
    )
    .map_err(|e| if is_unique_violation(&e) { duplicate_name(&name) } else { e.into() })?;
    get(conn, id)
}

/// Mueve una carpeta bajo otro padre (o a la raíz) con control optimista.
pub fn move_to(
    conn: &Connection,
    id: &str,
    target_parent_id: Option<&str>,
    expected_version: i64,
) -> AppResult<Folder> {
    let folder = get(conn, id)?;
    if let Some(target) = target_parent_id {
        if target == id {
            return Err(AppError::validation("Una carpeta no puede ser su propio padre"));
        }
        get(conn, target).map_err(|_| AppError::validation("La carpeta de destino no existe"))?;
        // El destino no puede estar dentro de la carpeta que se mueve.
        let creates_cycle: bool = conn.query_row(
            "WITH RECURSIVE up(id, parent_id) AS (
                 SELECT id, parent_id FROM folders WHERE id = ?1
                 UNION
                 SELECT f.id, f.parent_id FROM folders f JOIN up ON f.id = up.parent_id
             )
             SELECT EXISTS(SELECT 1 FROM up WHERE id = ?2)",
            params![target, id],
            |r| r.get(0),
        )?;
        if creates_cycle {
            return Err(AppError::validation(
                "No se puede mover una carpeta dentro de sí misma o de una subcarpeta suya",
            ));
        }
    }
    let changed = conn
        .execute(
            "UPDATE folders SET parent_id = ?2, version = version + 1, updated_at = ?3
             WHERE id = ?1 AND version = ?4 AND deleted_at IS NULL",
            params![id, target_parent_id, now(), expected_version],
        )
        .map_err(|e| if is_unique_violation(&e) { duplicate_name(&folder.name) } else { e.into() })?;
    if changed == 0 {
        return Err(AppError::conflict(format!(
            "La carpeta cambió mientras tanto (versión esperada {expected_version}, actual {})",
            folder.version
        )));
    }
    get(conn, id)
}

/// Nombre libre entre los hermanos vivos: "Nombre", "Nombre (2)", ...
pub fn unique_name(conn: &Connection, parent_id: Option<&str>, name: &str, exclude_id: &str) -> AppResult<String> {
    let taken = |candidate: &str| -> AppResult<bool> {
        Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE deleted_at IS NULL AND id <> ?3
               AND ifnull(parent_id, '') = ifnull(?1, '') AND name = ?2 COLLATE NOCASE)",
            params![parent_id, candidate, exclude_id],
            |r| r.get(0),
        )?)
    };
    if !taken(name)? {
        return Ok(name.to_string());
    }
    (2..)
        .map(|n| format!("{name} ({n})"))
        .find_map(|c| match taken(&c) {
            Ok(false) => Some(Ok(c)),
            Ok(true) => None,
            Err(e) => Some(Err(e)),
        })
        .expect("rango infinito")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn create_rejects_duplicate_siblings_case_insensitive() {
        let conn = open_in_memory().unwrap();
        create(&conn, "Rust", None).unwrap();
        let err = create(&conn, "rust", None).unwrap_err();
        assert_eq!(err.code(), "CONFLICT");
        let parent = create(&conn, "Lenguajes", None).unwrap();
        create(&conn, "Rust", Some(&parent.id)).unwrap();
    }

    #[test]
    fn move_prevents_cycles_and_checks_version() {
        let conn = open_in_memory().unwrap();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", Some(&a.id)).unwrap();
        let c = create(&conn, "C", Some(&b.id)).unwrap();
        assert_eq!(move_to(&conn, &a.id, Some(&c.id), a.version).unwrap_err().code(), "VALIDATION");
        assert_eq!(move_to(&conn, &c.id, None, 99).unwrap_err().code(), "CONFLICT");
        let moved = move_to(&conn, &c.id, None, c.version).unwrap();
        assert_eq!(moved.parent_id, None);
        assert_eq!(moved.version, c.version + 1);
    }

    #[test]
    fn tree_nests_and_counts() {
        let conn = open_in_memory().unwrap();
        let a = create(&conn, "A", None).unwrap();
        create(&conn, "B", Some(&a.id)).unwrap();
        crate::repo::notes::create(
            &conn,
            &crate::models::CreateNote { title: "n".into(), body: String::new(), folder_id: Some(a.id.clone()) },
        )
        .unwrap();
        let t = tree(&conn).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].note_count, 1);
        assert_eq!(t[0].children[0].folder.name, "B");
    }
}
