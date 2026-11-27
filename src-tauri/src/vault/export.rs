use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;

use regex::Captures;
use rusqlite::Connection;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::attachments::BODY_REF;
use crate::error::{AppError, AppResult};
use crate::models::ExportReport;
use crate::util::new_id;
use crate::vault::sanitize_segment;

pub const ATTACHMENTS_DIR: &str = "_attachments";
pub const TRASH_DIR: &str = "_papelera";

struct FolderRow {
    parent_id: Option<String>,
    name: String,
    deleted: bool,
}

struct NoteRow {
    id: String,
    folder_id: Option<String>,
    title: String,
    body: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

fn folder_path<'a>(folders: &'a HashMap<String, FolderRow>, mut id: Option<&'a str>) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    while let Some(fid) = id {
        if !seen.insert(fid) {
            break;
        }
        let Some(f) = folders.get(fid) else { break };
        names.push(sanitize_segment(&f.name, "Sin nombre"));
        id = f.parent_id.as_deref();
    }
    names.reverse();
    names
}

fn json(s: &str) -> String {
    serde_json::to_string(s).expect("string serializable")
}

/// Exporta el vault a un zip de Markdown: una carpeta por carpeta, un `.md`
/// por nota con frontmatter (id, título, fechas, etiquetas) y las imágenes en
/// `_attachments/`. Se escribe en un temporal y se renombra al final.
pub fn export_zip(conn: &Connection, attachments_dir: &Path, dest: &Path, include_trash: bool) -> AppResult<ExportReport> {
    let folders: HashMap<String, FolderRow> = {
        let mut stmt = conn.prepare("SELECT id, parent_id, name, deleted_at IS NOT NULL FROM folders")?;
        stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, FolderRow { parent_id: r.get(1)?, name: r.get(2)?, deleted: r.get(3)? }))
        })?
        .collect::<Result<_, _>>()?
    };
    let notes: Vec<NoteRow> = {
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, title, body, created_at, updated_at, deleted_at FROM notes
             WHERE ?1 OR deleted_at IS NULL ORDER BY title COLLATE NOCASE",
        )?;
        stmt.query_map([include_trash], |r| {
            Ok(NoteRow {
                id: r.get(0)?,
                folder_id: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
                deleted_at: r.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?
    };
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT nt.note_id, t.name FROM note_tags nt JOIN tags t ON t.id = nt.tag_id ORDER BY t.name",
        )?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (note, tag) = row?;
            tags.entry(note).or_default().push(tag);
        }
    }

    let parent = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err(AppError::validation("La carpeta de destino no existe"));
    }
    let tmp = parent.join(format!(".notas-export-{}.tmp", new_id()));
    let result = write_zip(&tmp, attachments_dir, &folders, &notes, &tags);
    match result {
        Ok(mut report) => {
            fs::rename(&tmp, dest).inspect_err(|_| {
                let _ = fs::remove_file(&tmp);
            })?;
            report.path = dest.display().to_string();
            Ok(report)
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn write_zip(
    tmp: &Path,
    attachments_dir: &Path,
    folders: &HashMap<String, FolderRow>,
    notes: &[NoteRow],
    tags: &HashMap<String, Vec<String>>,
) -> AppResult<ExportReport> {
    let file = fs::File::create(tmp)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut report = ExportReport::default();

    // Carpetas vivas como entradas explícitas: así se conservan las vacías.
    let mut dirs: Vec<String> = folders
        .iter()
        .filter(|(_, f)| !f.deleted)
        .map(|(id, _)| folder_path(folders, Some(id)).join("/"))
        .collect();
    dirs.sort();
    dirs.dedup();
    for d in &dirs {
        zip.add_directory(format!("{d}/"), opts)?;
    }
    report.folders = dirs.len();

    let mut used_paths: HashSet<String> = HashSet::new();
    let mut used_attachments: HashSet<String> = HashSet::new();
    for n in notes {
        let mut segments = folder_path(folders, n.folder_id.as_deref());
        if n.deleted_at.is_some() {
            segments.insert(0, TRASH_DIR.to_string());
        }
        let dir = segments.join("/");
        let stem = sanitize_segment(&n.title, "Sin título");
        let mut rel = String::new();
        for i in 1.. {
            let file = if i == 1 { format!("{stem}.md") } else { format!("{stem} ({i}).md") };
            rel = if dir.is_empty() { file } else { format!("{dir}/{file}") };
            if used_paths.insert(rel.to_lowercase()) {
                break;
            }
        }

        let up = "../".repeat(segments.len());
        let body = BODY_REF.replace_all(&n.body, |c: &Captures| {
            used_attachments.insert(c[1].to_string());
            format!("]({up}{ATTACHMENTS_DIR}/{})", &c[1])
        });
        let note_tags = tags.get(&n.id).cloned().unwrap_or_default();
        let mut front = format!(
            "---\nid: {}\ntitle: {}\ncreated: {}\nupdated: {}\ntags: {}\n",
            json(&n.id),
            json(&n.title),
            json(&n.created_at),
            json(&n.updated_at),
            serde_json::to_string(&note_tags).expect("tags serializable"),
        );
        if let Some(d) = &n.deleted_at {
            front.push_str(&format!("deleted: {}\n", json(d)));
        }
        front.push_str("---\n\n");

        zip.start_file(rel, opts)?;
        zip.write_all(front.as_bytes())?;
        zip.write_all(body.as_bytes())?;
        if !body.ends_with('\n') {
            zip.write_all(b"\n")?;
        }
        report.notes += 1;
    }

    let mut names: Vec<_> = used_attachments.into_iter().collect();
    names.sort();
    for name in names {
        let Ok(path) = crate::attachments::path_for(attachments_dir, &name) else { continue };
        let Ok(bytes) = fs::read(path) else { continue };
        zip.start_file(format!("{ATTACHMENTS_DIR}/{name}"), SimpleFileOptions::default().compression_method(CompressionMethod::Stored))?;
        zip.write_all(&bytes)?;
        report.attachments += 1;
    }

    let file = zip.finish()?;
    file.sync_all()?;
    Ok(report)
}
