use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::Connection;
use zip::ZipArchive;

use crate::attachments;
use crate::error::{AppError, AppResult};
use crate::models::{ImportReport, Note};
use crate::repo::{notes, tags};
use crate::util::{new_id, now, validate_title};
use crate::vault::{
    basename, dedupe_tags, normalize_ts, note_exists, note_id_exists, parse_frontmatter, rewrite_image_refs, FolderPaths,
};

const MAX_NOTE_BYTES: u64 = 10 * 1024 * 1024;
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

/// Sección de relaciones del export de cogni-vault cuando no tenía ninguna.
static EMPTY_CONNECTIONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n*## Conexiones\s*\n+No hay conexiones directas\.\s*$").unwrap());

struct MdEntry {
    dirs: Vec<String>,
    stem: String,
    content: String,
}

fn split_path(name: &str) -> Vec<String> {
    name.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".").map(str::to_string).collect()
}

fn hidden(parts: &[String]) -> bool {
    parts.iter().any(|p| p.starts_with('.') || p == "__MACOSX" || p == "..")
}

/// Importa un zip de Markdown (export de esta app, de cogni-vault u
/// Obsidian). La estructura de carpetas del zip se recrea tal cual. Las notas
/// que ya existen (mismo id, o mismo título en la misma carpeta) se omiten,
/// así que importar dos veces el mismo zip no duplica nada.
pub fn import_zip(conn: &Connection, attachments_dir: &Path, src: &Path) -> AppResult<ImportReport> {
    let mut archive = ZipArchive::new(fs::File::open(src)?)?;
    let mut report = ImportReport::default();
    let mut images: HashMap<String, String> = HashMap::new();
    let mut md: Vec<MdEntry> = Vec::new();
    let mut dirs: Vec<Vec<String>> = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name()?.into_owned();
        let parts = split_path(&entry_name);
        if parts.is_empty() || hidden(&parts) {
            continue;
        }
        if entry.is_dir() {
            if parts[0] != super::export::ATTACHMENTS_DIR && parts[0] != super::export::TRASH_DIR {
                dirs.push(parts);
            }
            continue;
        }
        let file_name = parts.last().unwrap().clone();
        let ext = file_name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        if ext == "md" {
            if entry.size() > MAX_NOTE_BYTES {
                continue;
            }
            let mut buf = Vec::new();
            entry.by_ref().take(MAX_NOTE_BYTES + 1).read_to_end(&mut buf)?;
            let stem = file_name[..file_name.len() - 3].to_string();
            md.push(MdEntry {
                dirs: parts[..parts.len() - 1].to_vec(),
                stem,
                content: String::from_utf8_lossy(&buf).into_owned(),
            });
        } else if IMAGE_EXTS.contains(&ext.as_str()) && entry.size() <= attachments::MAX_BYTES as u64 {
            let mut buf = Vec::new();
            entry.by_ref().take(attachments::MAX_BYTES as u64 + 1).read_to_end(&mut buf)?;
            if let Ok((stored, created)) = attachments::save_named(attachments_dir, &file_name, &buf) {
                if created {
                    report.attachments_imported += 1;
                }
                images.insert(file_name, stored);
            }
        }
    }

    if md.is_empty() && dirs.is_empty() {
        return Err(AppError::validation("El zip no contiene notas Markdown"));
    }

    let tx = conn.unchecked_transaction()?;
    let mut paths = FolderPaths::new();
    for d in &dirs {
        paths.ensure(&tx, d)?;
    }

    for entry in md {
        let (fm, body) = parse_frontmatter(&entry.content);
        let mut dirs = entry.dirs;
        let in_trash_dir = dirs
            .first()
            .is_some_and(|d| d == super::export::TRASH_DIR || d.eq_ignore_ascii_case("papelera"));
        if in_trash_dir {
            dirs.remove(0);
        }
        let deleted_at = fm
            .get("deleted")
            .or(fm.get("deleted_at"))
            .map(|d| normalize_ts(Some(d)))
            .or_else(|| in_trash_dir.then(now));

        let folder_id = paths.ensure(&tx, &dirs)?;
        let raw_title = fm.get("original_title").or(fm.get("title")).unwrap_or(&entry.stem);
        let title = validate_title(raw_title)
            .or_else(|_| validate_title(&entry.stem))
            .unwrap_or_else(|_| "Sin título".to_string());

        let given_id = fm.get("id").filter(|id| id.len() <= 100).map(str::to_string);
        if given_id.as_deref().map(|id| note_id_exists(&tx, id)).transpose()?.unwrap_or(false)
            || note_exists(&tx, folder_id.as_deref(), &title)?
        {
            report.notes_skipped += 1;
            continue;
        }

        let body = rewrite_image_refs(&body, |r| {
            if r.starts_with("http://") || r.starts_with("https://") || r.starts_with("data:") {
                return None;
            }
            images.get(basename(r)).cloned()
        });
        let body = EMPTY_CONNECTIONS.replace(&body, "").trim_end().to_string();

        let created_at = normalize_ts(fm.get("created").or(fm.get("date")));
        let note = Note {
            id: given_id.unwrap_or_else(new_id),
            folder_id,
            title,
            body,
            version: 1,
            updated_at: fm.get("updated").map(|u| normalize_ts(Some(u))).unwrap_or_else(|| created_at.clone()),
            created_at,
            deleted_at: deleted_at.clone(),
        };
        notes::insert_raw(&tx, &note)?;
        for tag in dedupe_tags(&fm.tags) {
            if let Ok(tag_id) = tags::find_or_create(&tx, &tag) {
                tags::attach(&tx, &note.id, &tag_id)?;
                report.tags_attached += 1;
            }
        }
        report.notes_created += 1;
        if deleted_at.is_some() {
            report.notes_trashed += 1;
        }
    }

    report.folders_created = paths.created;
    tx.commit()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::models::CreateNote;
    use crate::repo::{folders, trash};
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nDATA";

    #[test]
    fn export_then_import_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let att = tmp.path().join("att");
        let src = open_in_memory().unwrap();
        let img = attachments::save(&att, PNG).unwrap();

        let a = folders::create(&src, "Proyectos", None).unwrap();
        let b = folders::create(&src, "Rust", Some(&a.id)).unwrap();
        folders::create(&src, "Vacía", None).unwrap();
        let n = notes::create(&src, &CreateNote {
            title: "Ownership: \"borrow\"".into(),
            body: format!("Ver [[Lifetimes]]\n\n![diagrama](attachments/{img})"),
            folder_id: Some(b.id.clone()),
        }).unwrap();
        tags::set_for_note(&src, &n.id, &["rust".into()]).unwrap();
        let gone = notes::create(&src, &CreateNote { title: "Borrada".into(), body: "x".into(), folder_id: None }).unwrap();
        trash::trash_note(&src, &gone.id).unwrap();

        let zip_path = tmp.path().join("vault.zip");
        let rep = crate::vault::export::export_zip(&src, &att, &zip_path, true).unwrap();
        assert_eq!((rep.notes, rep.attachments), (2, 1));

        let dst = open_in_memory().unwrap();
        let att2 = tmp.path().join("att2");
        let imp = import_zip(&dst, &att2, &zip_path).unwrap();
        assert_eq!(imp.notes_created, 2);
        assert_eq!(imp.notes_trashed, 1);
        assert_eq!(imp.attachments_imported, 1);
        assert!(folders::list(&dst).unwrap().iter().any(|f| f.name == "Vacía"));

        let got = notes::get(&dst, &n.id).unwrap();
        assert_eq!(got.title, n.title);
        assert_eq!(got.body, n.body, "las rutas de imagen vuelven a attachments/");
        assert_eq!(got.created_at, n.created_at);
        let parent = folders::get(&dst, got.folder_id.as_deref().unwrap()).unwrap();
        assert_eq!(parent.name, "Rust");
        assert_eq!(tags::for_note(&dst, &n.id).unwrap()[0].name, "rust");
        assert!(notes::get(&dst, &gone.id).unwrap().deleted_at.is_some());
        assert!(att2.join(&img).exists());

        // Importar de nuevo no duplica.
        let again = import_zip(&dst, &att2, &zip_path).unwrap();
        assert_eq!((again.notes_created, again.notes_skipped), (0, 2));
    }

    #[test]
    fn imports_cogni_vault_export_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("cv.zip");
        {
            let mut z = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
            let o = SimpleFileOptions::default();
            z.start_file("Ciencia/Física/DomainCore.md", o).unwrap();
            z.write_all(
                b"---\ntitle: \"DomainCore\"\noriginal_title: \"Domain Core\"\ntags:\n  - Nota\n  - Fisica\ndate: 2025-03-01\n---\nTexto ![img](/api/v1/images/a.png)\n\n## Conexiones\nNo hay conexiones directas.\n",
            ).unwrap();
            z.start_file("Papelera/Ciencia/Vieja.md", o).unwrap();
            z.write_all(b"---\ntitle: \"Vieja\"\ndeleted_at: \"2025-04-01T10:00:00.000Z\"\n---\nx").unwrap();
            z.start_file("_attachments/a.png", o).unwrap();
            z.write_all(PNG).unwrap();
            z.finish().unwrap();
        }
        let conn = open_in_memory().unwrap();
        let rep = import_zip(&conn, &tmp.path().join("att"), &zip_path).unwrap();
        assert_eq!((rep.notes_created, rep.notes_trashed, rep.folders_created), (2, 1, 2));
        let live = notes::list(&conn).unwrap();
        assert_eq!(live.len(), 1);
        let n = notes::get(&conn, &live[0].id).unwrap();
        assert_eq!(n.title, "Domain Core");
        assert_eq!(n.body, "Texto ![img](attachments/a.png)");
        assert_eq!(n.created_at, "2025-03-01T00:00:00.000Z");
        let t: Vec<_> = tags::for_note(&conn, &n.id).unwrap().into_iter().map(|t| t.name).collect();
        assert_eq!(t, ["fisica"]);
    }
}
