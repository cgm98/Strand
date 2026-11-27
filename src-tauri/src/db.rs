use std::path::Path;

use rusqlite::Connection;

use crate::error::AppResult;

/// Migraciones en orden. Cada una se aplica una sola vez dentro de una
/// transacción y deja `PRAGMA user_version` = su índice + 1.
/// Nunca editar una migración publicada: añadir una nueva al final.
const MIGRATIONS: &[&str] = &[MIGRATION_001];

const MIGRATION_001: &str = r#"
CREATE TABLE trash_batches (
    id              TEXT PRIMARY KEY,
    root_folder_id  TEXT NOT NULL,
    root_name       TEXT NOT NULL,
    item_count      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    restored_at     TEXT,
    purged_at       TEXT
);

CREATE TABLE folders (
    id              TEXT PRIMARY KEY,
    parent_id       TEXT REFERENCES folders(id),
    name            TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 150),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT,
    trash_batch_id  TEXT REFERENCES trash_batches(id)
);
CREATE INDEX idx_folders_parent ON folders(parent_id);
CREATE INDEX idx_folders_batch ON folders(trash_batch_id);
-- Nombres únicos entre hermanos vivos (la raíz cuenta como un padre más).
CREATE UNIQUE INDEX idx_folders_unique_name
    ON folders(ifnull(parent_id, ''), name COLLATE NOCASE)
    WHERE deleted_at IS NULL;

-- `seq` es el rowid estable que usa el índice FTS (un rowid implícito
-- podría renumerarse con VACUUM y desincronizar la búsqueda).
CREATE TABLE notes (
    seq             INTEGER PRIMARY KEY,
    id              TEXT NOT NULL UNIQUE,
    folder_id       TEXT REFERENCES folders(id) ON DELETE SET NULL,
    title           TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    title_key       TEXT NOT NULL,
    body            TEXT NOT NULL DEFAULT '',
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT,
    trash_batch_id  TEXT REFERENCES trash_batches(id)
);
CREATE INDEX idx_notes_folder ON notes(folder_id);
CREATE INDEX idx_notes_title_key ON notes(title_key);
CREATE INDEX idx_notes_updated ON notes(updated_at);
CREATE INDEX idx_notes_batch ON notes(trash_batch_id);

CREATE TABLE note_versions (
    id          TEXT PRIMARY KEY,
    note_id     TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_note_versions_note ON note_versions(note_id, created_at);

CREATE TABLE tags (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    color       TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE note_tags (
    note_id     TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id      TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (note_id, tag_id)
) WITHOUT ROWID;
CREATE INDEX idx_note_tags_tag ON note_tags(tag_id);

-- Enlaces [[...]] salientes. Se resuelven por clave de título al consultar,
-- así un enlace a una nota que aún no existe se conecta solo al crearla.
CREATE TABLE note_links (
    source_id    TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    target_key   TEXT NOT NULL,
    target_title TEXT NOT NULL,
    PRIMARY KEY (source_id, target_key)
) WITHOUT ROWID;
CREATE INDEX idx_note_links_target ON note_links(target_key);

CREATE VIRTUAL TABLE notes_fts USING fts5(
    title,
    body,
    content = 'notes',
    content_rowid = 'seq',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER notes_fts_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, body) VALUES (new.seq, new.title, new.body);
END;
CREATE TRIGGER notes_fts_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body)
    VALUES ('delete', old.seq, old.title, old.body);
END;
CREATE TRIGGER notes_fts_au AFTER UPDATE OF title, body ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body)
    VALUES ('delete', old.seq, old.title, old.body);
    INSERT INTO notes_fts(rowid, title, body) VALUES (new.seq, new.title, new.body);
END;
"#;

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> AppResult<()> {
    // journal_mode devuelve una fila, por eso query_row y no execute.
    let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = FULL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > MIGRATIONS.len() as i64 {
        return Err(crate::error::AppError::Internal(format!(
            "La base de datos es de una versión más nueva de la app (esquema {current})"
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        drop(open(&path).unwrap());
        let conn = open(&path).unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
    }
}
