//! Migración directa desde la base SQLite de cogni-vault (`db/cogni_vault.db`).
//!
//! Correspondencia:
//! - `cerebros` (espacios) → una carpeta raíz por espacio.
//! - `folders` → subcarpetas dentro de la carpeta de su espacio.
//! - Notas sin espacio → carpetas `Área/Dominio/Tema`, como hacía su export.
//! - `concepts` → notas (mismo id; `intuition` y `code_snippet` se añaden
//!   al cuerpo). Las de la papelera llegan a la papelera.
//! - `concept_versions` → historial; `tags`/`concept_tags` → etiquetas.
//! - `relations` → sección `## Conexiones` con `[[enlaces]]`.
//! - Imágenes `/api/(v1/)images/x` → copiadas a los adjuntos.
//!
//! Se descartan taxonomía, capa epistémica, repasos/flashcards y vectores.
//! La base de origen se abre en solo lectura.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};

use crate::attachments;
use crate::error::{AppError, AppResult};
use crate::models::{ImportReport, Note};
use crate::repo::{notes, tags};
use crate::util::validate_title;
use crate::vault::{basename, normalize_ts, note_id_exists, rewrite_image_refs, FolderPaths};

struct Legacy<'a> {
    conn: &'a Connection,
}

impl Legacy<'_> {
    fn has_table(&self, name: &str) -> AppResult<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [name],
            |r| r.get(0),
        )?)
    }

    fn columns(&self, table: &str) -> AppResult<HashSet<String>> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
        let cols = stmt.query_map([], |r| r.get::<_, String>(1))?.collect::<Result<_, _>>()?;
        Ok(cols)
    }

    /// `SELECT a, b, NULL AS c FROM t`: tolera columnas que una versión
    /// antigua de cogni-vault no tenía.
    fn select(&self, table: &str, wanted: &[&str]) -> AppResult<String> {
        let have = self.columns(table)?;
        let cols: Vec<String> = wanted
            .iter()
            .map(|c| if have.contains(*c) { format!("\"{c}\"") } else { format!("NULL AS \"{c}\"") })
            .collect();
        Ok(format!("SELECT {} FROM \"{table}\"", cols.join(", ")))
    }

    fn name_map(&self, table: &str, extra: &str) -> AppResult<HashMap<String, (String, Option<String>)>> {
        if !self.has_table(table)? {
            return Ok(HashMap::new());
        }
        let sql = self.select(table, &["id", "name", extra])?;
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, (r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get(2)?)))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

struct Concept {
    id: String,
    title: String,
    explanation: String,
    intuition: Option<String>,
    code_snippet: Option<String>,
    domain: Option<String>,
    topic_id: Option<String>,
    workspace_id: Option<String>,
    folder_id: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    deleted_at: Option<String>,
}

struct LegacyFolder {
    workspace_id: String,
    parent_id: Option<String>,
    name: String,
}

pub fn import_legacy_db(
    conn: &Connection,
    attachments_dir: &Path,
    legacy_path: &Path,
    uploads_dir: Option<&Path>,
) -> AppResult<ImportReport> {
    let legacy_conn = Connection::open_with_flags(legacy_path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    let legacy = Legacy { conn: &legacy_conn };
    if !legacy.has_table("concepts")? {
        return Err(AppError::validation("El archivo no parece una base de datos de cogni-vault"));
    }

    let workspaces = legacy.name_map("cerebros", "slug")?;
    let areas = legacy.name_map("areas", "cerebro_id")?;
    let domains = legacy.name_map("domains", "area_id")?;
    let topics = legacy.name_map("topics", "domain_id")?;

    let legacy_folders: HashMap<String, LegacyFolder> = if legacy.has_table("folders")? {
        let sql = legacy.select("folders", &["id", "workspace_id", "parent_id", "name"])? + " WHERE deleted_at IS NULL";
        let mut stmt = legacy_conn.prepare(&sql)?;
        stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, LegacyFolder { workspace_id: r.get(1)?, parent_id: r.get(2)?, name: r.get(3)? }))
        })?
        .collect::<Result<_, _>>()?
    } else {
        HashMap::new()
    };

    let concepts: Vec<Concept> = {
        let sql = legacy.select(
            "concepts",
            &[
                "id", "title", "explanation", "intuition", "code_snippet", "domain", "topic_id", "workspace_id",
                "folder_id", "created_at", "updated_at", "deleted_at",
            ],
        )?;
        let mut stmt = legacy_conn.prepare(&sql)?;
        stmt.query_map([], |r| {
            Ok(Concept {
                id: r.get(0)?,
                title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                explanation: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                intuition: r.get(3)?,
                code_snippet: r.get(4)?,
                domain: r.get(5)?,
                topic_id: r.get(6)?,
                workspace_id: r.get(7)?,
                folder_id: r.get(8)?,
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
                deleted_at: r.get(11)?,
            })
        })?
        .collect::<Result<_, _>>()?
    };
    let titles: HashMap<&str, &str> = concepts.iter().map(|c| (c.id.as_str(), c.title.as_str())).collect();

    let mut relations: HashMap<String, Vec<(String, String)>> = HashMap::new();
    if legacy.has_table("relations")? {
        let mut stmt = legacy_conn.prepare(&legacy.select("relations", &["from_concept_id", "to_concept_id", "relation_type"])?)?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))? {
            let (from, to, kind) = row?;
            if let Some(target) = titles.get(to.as_str()) {
                relations.entry(from).or_default().push((target.to_string(), kind.unwrap_or_else(|| "related_to".into())));
            }
        }
    }

    let mut legacy_tags: HashMap<String, Vec<String>> = HashMap::new();
    if legacy.has_table("tags")? && legacy.has_table("concept_tags")? {
        let mut stmt = legacy_conn.prepare("SELECT ct.concept_id, t.name FROM concept_tags ct JOIN tags t ON t.id = ct.tag_id")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (c, t) = row?;
            legacy_tags.entry(c).or_default().push(t);
        }
    }
    let has_versions = legacy.has_table("concept_versions")?;

    let upload_dirs = candidate_upload_dirs(legacy_path, uploads_dir);
    let mut image_cache: HashMap<String, Option<String>> = HashMap::new();
    let mut report = ImportReport::default();

    let tx = conn.unchecked_transaction()?;
    let mut paths = FolderPaths::new();

    for c in &concepts {
        if note_id_exists(&tx, &c.id)? {
            report.notes_skipped += 1;
            continue;
        }

        let segments = if let Some(ws) = c.workspace_id.as_deref().filter(|w| !w.is_empty()) {
            let mut segs = vec![workspaces.get(ws).map(|w| w.0.clone()).unwrap_or_else(|| "Espacio".into())];
            segs.extend(legacy_folder_path(&legacy_folders, c.folder_id.as_deref(), ws));
            segs
        } else {
            taxonomy_path(c, &areas, &domains, &topics)
        };
        let folder_id = paths.ensure(&tx, &segments)?;

        let mut body = c.explanation.trim_end().to_string();
        if let Some(intuition) = c.intuition.as_deref().map(str::trim).filter(|i| !i.is_empty() && !body.contains(*i)) {
            body.push_str(&format!("\n\n### Intuición\n> {intuition}"));
        }
        if let Some(code) = c.code_snippet.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            body.push_str(&format!("\n\n```\n{code}\n```"));
        }
        if let Some(rels) = relations.get(&c.id) {
            body.push_str("\n\n## Conexiones\n");
            for (target, kind) in rels {
                body.push_str(&format!("- [[{target}]] ({kind})\n"));
            }
        }
        let body = rewrite_image_refs(body.trim_start(), |r| {
            legacy_image(r, &upload_dirs, attachments_dir, &mut image_cache, &mut report)
        });

        let title = validate_title(&c.title).unwrap_or_else(|_| {
            let t: String = c.title.trim().chars().take(300).collect();
            if crate::util::title_key(&t).is_empty() { "Sin título".into() } else { t }
        });
        let created_at = normalize_ts(c.created_at.as_deref());
        let note = Note {
            id: c.id.clone(),
            folder_id,
            title,
            body,
            version: 1,
            updated_at: c.updated_at.as_deref().map(|u| normalize_ts(Some(u))).unwrap_or_else(|| created_at.clone()),
            created_at,
            deleted_at: c.deleted_at.as_deref().map(|d| normalize_ts(Some(d))),
        };
        notes::insert_raw(&tx, &note)?;
        report.notes_created += 1;
        if note.deleted_at.is_some() {
            report.notes_trashed += 1;
        }

        if has_versions {
            let mut stmt = legacy_conn.prepare_cached(
                "SELECT id, title, explanation, created_at FROM concept_versions WHERE concept_id = ?1",
            )?;
            let versions = stmt
                .query_map([&c.id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for (id, vtitle, vbody, vdate) in versions {
                report.versions_imported += tx.execute(
                    "INSERT OR IGNORE INTO note_versions (id, note_id, title, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        id,
                        note.id,
                        vtitle.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| note.title.clone()),
                        vbody.unwrap_or_default(),
                        normalize_ts(vdate.as_deref())
                    ],
                )?;
            }
        }

        for tag in crate::vault::dedupe_tags(legacy_tags.get(&c.id).map(Vec::as_slice).unwrap_or(&[])) {
            if let Ok(tag_id) = tags::find_or_create(&tx, &tag) {
                tags::attach(&tx, &note.id, &tag_id)?;
                report.tags_attached += 1;
            }
        }
    }

    // Carpetas sin notas: también se conservan.
    for (id, f) in &legacy_folders {
        let mut segs = vec![workspaces.get(&f.workspace_id).map(|w| w.0.clone()).unwrap_or_else(|| "Espacio".into())];
        segs.extend(legacy_folder_path(&legacy_folders, Some(id), &f.workspace_id));
        paths.ensure(&tx, &segs)?;
    }

    report.folders_created = paths.created;
    tx.commit()?;
    Ok(report)
}

fn legacy_folder_path<'a>(folders: &'a HashMap<String, LegacyFolder>, mut id: Option<&'a str>, workspace: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    while let Some(fid) = id {
        let Some(f) = folders.get(fid).filter(|f| f.workspace_id == workspace) else { break };
        if !seen.insert(fid) {
            break;
        }
        names.push(f.name.clone());
        id = f.parent_id.as_deref();
    }
    names.reverse();
    names
}

type NameMap = HashMap<String, (String, Option<String>)>;

fn taxonomy_path(c: &Concept, areas: &NameMap, domains: &NameMap, topics: &NameMap) -> Vec<String> {
    let domain = c.domain.as_deref().and_then(|d| domains.get(d));
    let area = domain.and_then(|(_, area_id)| area_id.as_deref()).and_then(|a| areas.get(a));
    let mut segs = vec![
        area.map(|a| a.0.clone()).unwrap_or_else(|| "Sin área".into()),
        domain
            .map(|d| d.0.clone())
            .or_else(|| c.domain.clone().filter(|d| !d.is_empty()))
            .unwrap_or_else(|| "Sin dominio".into()),
    ];
    if let Some(topic) = c.topic_id.as_deref().and_then(|t| topics.get(t)) {
        segs.push(topic.0.clone());
    }
    segs
}

fn candidate_upload_dirs(legacy_path: &Path, explicit: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = explicit.into_iter().map(Path::to_path_buf).collect();
    if let Some(db_dir) = legacy_path.parent() {
        dirs.push(db_dir.join("uploads"));
        if let Some(root) = db_dir.parent() {
            dirs.push(root.join("uploads"));
            dirs.push(root.join("backend").join("uploads"));
        }
    }
    dirs
}

fn legacy_image(
    reference: &str,
    dirs: &[PathBuf],
    attachments_dir: &Path,
    cache: &mut HashMap<String, Option<String>>,
    report: &mut ImportReport,
) -> Option<String> {
    let is_legacy = reference.starts_with("/api/images/") || reference.starts_with("/api/v1/images/");
    if !is_legacy {
        return None;
    }
    let name = basename(reference).to_string();
    if let Some(hit) = cache.get(&name) {
        return hit.clone();
    }
    let stored = dirs
        .iter()
        .filter(|_| attachments::is_safe_name(&name))
        .find_map(|d| fs::read(d.join(&name)).ok())
        .and_then(|bytes| attachments::save_named(attachments_dir, &name, &bytes).ok());
    match &stored {
        Some((_, true)) => report.attachments_imported += 1,
        Some((_, false)) => {}
        None => report.attachments_missing += 1,
    }
    let result = stored.map(|(n, _)| n);
    cache.insert(name, result.clone());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::repo::{folders, trash};

    /// Base mínima con la forma de cogni-vault (columnas reales del repo).
    fn legacy_fixture(dir: &Path) -> PathBuf {
        let db_dir = dir.join("db");
        fs::create_dir_all(&db_dir).unwrap();
        fs::create_dir_all(dir.join("uploads")).unwrap();
        fs::write(dir.join("uploads").join("img-1.png"), b"\x89PNG\r\n\x1a\nX").unwrap();
        let path = db_dir.join("cogni_vault.db");
        let c = Connection::open(&path).unwrap();
        c.execute_batch(
            r#"
            CREATE TABLE cerebros (id TEXT PRIMARY KEY, slug TEXT, name TEXT, user_id TEXT, created_at TEXT, updated_at TEXT);
            CREATE TABLE areas (id TEXT PRIMARY KEY, cerebro_id TEXT, user_id TEXT, name TEXT);
            CREATE TABLE domains (id TEXT PRIMARY KEY, user_id TEXT, name TEXT, color TEXT, area_id TEXT);
            CREATE TABLE topics (id TEXT PRIMARY KEY, user_id TEXT, domain_id TEXT, name TEXT);
            CREATE TABLE folders (id TEXT PRIMARY KEY, workspace_id TEXT, parent_id TEXT, name TEXT, sort_order INTEGER,
                version INTEGER, created_at TEXT, updated_at TEXT, deleted_at TEXT, trash_batch_id TEXT);
            CREATE TABLE concepts (id TEXT PRIMARY KEY, user_id TEXT, title TEXT, explanation TEXT, intuition TEXT,
                domain TEXT, topic_id TEXT, strength REAL, created_at TEXT, updated_at TEXT, deleted_at TEXT,
                workspace_id TEXT, folder_id TEXT);
            CREATE TABLE concept_versions (id TEXT PRIMARY KEY, concept_id TEXT, title TEXT, explanation TEXT, created_at TEXT);
            CREATE TABLE relations (id TEXT PRIMARY KEY, from_concept_id TEXT, to_concept_id TEXT, relation_type TEXT, created_at TEXT);
            CREATE TABLE tags (id TEXT PRIMARY KEY, workspace_id TEXT, name TEXT, color TEXT, created_at TEXT);
            CREATE TABLE concept_tags (concept_id TEXT, tag_id TEXT, created_at TEXT);

            INSERT INTO cerebros VALUES ('w1', 'principal', 'Principal', 'u', '', '');
            INSERT INTO folders VALUES ('f1', 'w1', NULL, 'Rust', 0, 1, '', '', NULL, NULL);
            INSERT INTO folders VALUES ('f2', 'w1', 'f1', 'Vacía', 0, 1, '', '', NULL, NULL);
            INSERT INTO areas VALUES ('a1', 'w1', 'u', 'Ciencia');
            INSERT INTO domains VALUES ('d1', 'u', 'Física', '#fff', 'a1');
            INSERT INTO topics VALUES ('t1', 'u', 'd1', 'Mecánica');

            INSERT INTO concepts VALUES ('c1', 'u', 'Ownership', 'Texto ![x](/api/v1/images/img-1.png) ![y](/api/images/falta.png)',
                'Cada valor tiene un dueño', 'd1', NULL, 0.7, '2025-01-01T00:00:00.000Z', '2025-01-02T00:00:00.000Z', NULL, 'w1', 'f1');
            INSERT INTO concepts VALUES ('c2', 'u', 'Inercia', 'Newton', NULL, 'd1', 't1', 0.7, '2025-01-01T00:00:00.000Z',
                '2025-01-01T00:00:00.000Z', NULL, NULL, NULL);
            INSERT INTO concepts VALUES ('c3', 'u', 'Borrada', 'x', NULL, 'sin-clasificar', NULL, 0.7, '2025-01-01T00:00:00.000Z',
                '2025-01-01T00:00:00.000Z', '2025-02-01T00:00:00.000Z', NULL, NULL);
            INSERT INTO concept_versions VALUES ('v1', 'c1', 'Ownership', 'antes', '2024-12-31T00:00:00.000Z');
            INSERT INTO relations VALUES ('r1', 'c1', 'c2', 'depends_on', '');
            INSERT INTO tags VALUES ('tg1', 'w1', 'rust', NULL, '');
            INSERT INTO concept_tags VALUES ('c1', 'tg1', '');
            "#,
        )
        .unwrap();
        path
    }

    #[test]
    fn migrates_cogni_vault_database() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy = legacy_fixture(tmp.path());
        let att = tmp.path().join("att");
        let conn = open_in_memory().unwrap();

        let rep = import_legacy_db(&conn, &att, &legacy, None).unwrap();
        assert_eq!(rep.notes_created, 3);
        assert_eq!(rep.notes_trashed, 1);
        assert_eq!(rep.versions_imported, 1);
        assert_eq!(rep.tags_attached, 1);
        assert_eq!((rep.attachments_imported, rep.attachments_missing), (1, 1));

        let c1 = notes::get(&conn, "c1").unwrap();
        assert!(c1.body.starts_with("Texto ![x](attachments/img-1.png) ![y](/api/images/falta.png)"));
        assert!(c1.body.contains("### Intuición\n> Cada valor tiene un dueño"));
        assert!(c1.body.contains("- [[Inercia]] (depends_on)"));
        assert_eq!(c1.created_at, "2025-01-01T00:00:00.000Z");
        let rust = folders::get(&conn, c1.folder_id.as_deref().unwrap()).unwrap();
        assert_eq!(rust.name, "Rust");
        assert_eq!(folders::get(&conn, rust.parent_id.as_deref().unwrap()).unwrap().name, "Principal");
        assert!(folders::list(&conn).unwrap().iter().any(|f| f.name == "Vacía"));

        let c2 = notes::get(&conn, "c2").unwrap();
        let mecanica = folders::get(&conn, c2.folder_id.as_deref().unwrap()).unwrap();
        assert_eq!(mecanica.name, "Mecánica");

        // La relación es ahora un [[enlace]] con backlink.
        assert_eq!(notes::backlinks(&conn, "c2").unwrap()[0].id, "c1");
        assert_eq!(notes::list_versions(&conn, "c1").unwrap().len(), 1);
        assert_eq!(trash::list(&conn).unwrap().notes[0].id, "c3");

        // Idempotente.
        let again = import_legacy_db(&conn, &att, &legacy, None).unwrap();
        assert_eq!((again.notes_created, again.notes_skipped), (0, 3));
    }
}
