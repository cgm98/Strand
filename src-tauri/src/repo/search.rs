use rusqlite::{params, Connection};

use crate::error::AppResult;
use crate::models::SearchHit;
use crate::util::{fts_query, HIGHLIGHT_END, HIGHLIGHT_START};

/// Búsqueda de texto completo (FTS5, BM25) sobre título y cuerpo de las
/// notas vivas. El título pesa 5 veces más que el cuerpo. Insensible a
/// mayúsculas y tildes ("arbol" encuentra "Árbol").
pub fn search(conn: &Connection, raw_query: &str, limit: u32) -> AppResult<Vec<SearchHit>> {
    let Some(query) = fts_query(raw_query) else { return Ok(Vec::new()) };
    let limit = limit.clamp(1, 200);
    let mut stmt = conn.prepare_cached(
        "SELECT n.id, n.folder_id, n.title, n.updated_at,
                snippet(notes_fts, 1, ?3, ?4, '…', 16),
                bm25(notes_fts, 5.0, 1.0) AS rank
         FROM notes_fts JOIN notes n ON n.seq = notes_fts.rowid
         WHERE notes_fts MATCH ?1 AND n.deleted_at IS NULL
         ORDER BY rank LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(
            params![query, limit, HIGHLIGHT_START.to_string(), HIGHLIGHT_END.to_string()],
            |r| {
                Ok(SearchHit {
                    id: r.get(0)?,
                    folder_id: r.get(1)?,
                    title: r.get(2)?,
                    updated_at: r.get(3)?,
                    snippet: r.get(4)?,
                    rank: r.get(5)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::models::{CreateNote, UpdateNote};
    use crate::repo::{notes, trash};

    #[test]
    fn finds_without_accents_prefixes_and_skips_trash() {
        let conn = open_in_memory().unwrap();
        let a = notes::create(&conn, &CreateNote { title: "Árbol binario".into(), body: "Estructura de datos".into(), folder_id: None }).unwrap();
        let b = notes::create(&conn, &CreateNote { title: "Otra".into(), body: "Un árbol en el cuerpo".into(), folder_id: None }).unwrap();

        let hits = search(&conn, "arbol", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, a.id, "coincidencia en título primero");
        assert!(hits[1].snippet.contains(HIGHLIGHT_START));

        assert_eq!(search(&conn, "estruc", 10).unwrap()[0].id, a.id);

        trash::trash_note(&conn, &b.id).unwrap();
        assert_eq!(search(&conn, "arbol", 10).unwrap().len(), 1);

        // El índice sigue a las ediciones.
        notes::update(&conn, &a.id, &UpdateNote { title: Some("Grafo".into()), body: Some("nodos".into()), expected_version: a.version }).unwrap();
        assert!(search(&conn, "arbol", 10).unwrap().is_empty());
        assert_eq!(search(&conn, "grafo nod", 10).unwrap().len(), 1);
        assert!(search(&conn, "\"OR (", 10).is_ok(), "la entrada del usuario no rompe la sintaxis FTS");
    }
}
