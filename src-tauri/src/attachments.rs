//! Imágenes adjuntas. Se guardan en `<datos>/attachments/` y las notas las
//! referencian con la ruta relativa `attachments/<nombre>`, que es Markdown
//! portable. El webview las carga por el protocolo `attachment://`.

use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::util::new_id;

pub const MAX_BYTES: usize = 20 * 1024 * 1024;
pub const BODY_PREFIX: &str = "attachments/";

static SAFE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,150}$").unwrap());

/// Referencia a un adjunto en el cuerpo de una nota: `](attachments/<nombre>)`.
/// La comparte `vault::export` para no mantener el mismo patrón dos veces.
pub static BODY_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\]\(attachments/([A-Za-z0-9][A-Za-z0-9._-]*)\)").unwrap());

fn collect_refs(body: &str, out: &mut HashSet<String>) {
    for c in BODY_REF.captures_iter(body) {
        out.insert(c[1].to_string());
    }
}

/// Adjuntos en uso: en el cuerpo de cualquier nota (incluidas las de la
/// papelera, que se pueden restaurar) y en cualquier versión guardada
/// (restaurar una versión vieja no debe quedarse sin sus imágenes).
fn referenced(conn: &Connection) -> AppResult<HashSet<String>> {
    let mut used = HashSet::new();
    let mut stmt = conn.prepare("SELECT body FROM notes")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        collect_refs(&row.get::<_, String>(0)?, &mut used);
    }
    let mut stmt = conn.prepare("SELECT body FROM note_versions")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        collect_refs(&row.get::<_, String>(0)?, &mut used);
    }
    Ok(used)
}

/// Borra de `dir` los archivos que ninguna nota ni versión referencia.
/// Devuelve los nombres borrados, ordenados.
pub fn prune_orphans(conn: &Connection, dir: &Path) -> AppResult<Vec<String>> {
    let used = referenced(conn)?;
    let mut removed = Vec::new();
    if !dir.is_dir() {
        return Ok(removed);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if used.contains(&name) {
            continue;
        }
        fs::remove_file(entry.path())?;
        removed.push(name);
    }
    removed.sort();
    Ok(removed)
}

pub fn is_safe_name(name: &str) -> bool {
    SAFE_NAME.is_match(name) && !name.contains("..")
}

/// Extensión según los bytes reales, no según lo que diga el cliente.
pub fn sniff_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_lowercase();
        let head = head.trim_start_matches('\u{feff}').trim_start();
        (head.starts_with("<svg") || (head.starts_with("<?xml") && head.contains("<svg"))).then_some("svg")
    }
}

pub fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Escritura atómica: archivo temporal + fsync + rename. Un corte de luz
/// deja el archivo viejo o el nuevo completo, nunca uno a medias.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!("tmp-{}", new_id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}

/// Guarda una imagen y devuelve su nombre (`img-<uuid>.<ext>`).
pub fn save(dir: &Path, bytes: &[u8]) -> AppResult<String> {
    if bytes.is_empty() {
        return Err(AppError::validation("La imagen está vacía"));
    }
    if bytes.len() > MAX_BYTES {
        return Err(AppError::validation("La imagen supera el máximo de 20 MB"));
    }
    let ext = sniff_ext(bytes)
        .ok_or_else(|| AppError::validation("Formato no admitido (png, jpg, gif, webp o svg)"))?;
    fs::create_dir_all(dir)?;
    let name = format!("img-{}.{ext}", new_id());
    write_atomic(&dir.join(&name), bytes)?;
    Ok(name)
}

/// Guarda `bytes` intentando conservar `preferred` como nombre. Si ya existe
/// un archivo idéntico lo reutiliza; si existe uno distinto, genera otro
/// nombre. Devuelve `(nombre, creado)`.
pub fn save_named(dir: &Path, preferred: &str, bytes: &[u8]) -> AppResult<(String, bool)> {
    if sniff_ext(bytes).is_none() || bytes.len() > MAX_BYTES {
        return Err(AppError::validation(format!("Adjunto no válido: {preferred}")));
    }
    fs::create_dir_all(dir)?;
    if is_safe_name(preferred) {
        let path = dir.join(preferred);
        if !path.exists() {
            write_atomic(&path, bytes)?;
            return Ok((preferred.to_string(), true));
        }
        if fs::read(&path)? == bytes {
            return Ok((preferred.to_string(), false));
        }
    }
    Ok((save(dir, bytes)?, true))
}

pub fn path_for(dir: &Path, name: &str) -> AppResult<PathBuf> {
    if !is_safe_name(name) {
        return Err(AppError::validation("Nombre de adjunto no válido"));
    }
    Ok(dir.join(name))
}

/// Respuesta del protocolo `attachment://localhost/<nombre>`.
pub fn serve(dir: &Path, uri_path: &str) -> tauri::http::Response<Cow<'static, [u8]>> {
    use tauri::http::{header, Response, StatusCode};
    let name = uri_path.trim_start_matches('/');
    let name = crate::util::percent_decode(name);
    let result = path_for(dir, &name).and_then(|p| Ok(fs::read(p)?));
    match result {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_for(&name))
            // Un SVG abierto directamente no podrá ejecutar scripts.
            .header(header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'")
            .header("X-Content-Type-Options", "nosniff")
            .body(Cow::Owned(bytes))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&b"not found"[..]))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n0000";

    #[test]
    fn rejects_unknown_and_traversal() {
        let dir = tempfile::tempdir().unwrap();
        assert!(save(dir.path(), b"hola").is_err());
        assert!(path_for(dir.path(), "../etc/passwd").is_err());
        assert!(path_for(dir.path(), "a/b.png").is_err());
        let name = save(dir.path(), PNG).unwrap();
        assert!(name.ends_with(".png"));
        assert_eq!(serve(dir.path(), &format!("/{name}")).status(), 200);
        assert_eq!(serve(dir.path(), "/..%2Fx").status(), 404);
    }

    #[test]
    fn prune_orphans_keeps_live_trashed_and_versioned_refs() {
        use crate::db::open_in_memory;
        use crate::models::CreateNote;

        let conn = open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        for name in ["live.png", "trashed.png", "old-version.png", "orphan.png"] {
            fs::write(dir.path().join(name), PNG).unwrap();
        }

        let live = crate::repo::notes::create(
            &conn,
            &CreateNote { title: "Viva".into(), body: "![a](attachments/live.png)".into(), folder_id: None },
        )
        .unwrap();
        let trashed = crate::repo::notes::create(
            &conn,
            &CreateNote { title: "En papelera".into(), body: "![a](attachments/trashed.png)".into(), folder_id: None },
        )
        .unwrap();
        crate::repo::trash::trash_note(&conn, &trashed.id).unwrap();
        // Una versión vieja puede referenciar una imagen que el cuerpo actual ya no usa.
        conn.execute(
            "INSERT INTO note_versions (id, note_id, title, body, created_at) VALUES ('v1', ?1, 'Viva', '![a](attachments/old-version.png)', '2020-01-01')",
            [&live.id],
        )
        .unwrap();

        let removed = prune_orphans(&conn, dir.path()).unwrap();
        assert_eq!(removed, vec!["orphan.png".to_string()]);
        assert!(dir.path().join("live.png").exists());
        assert!(dir.path().join("trashed.png").exists());
        assert!(dir.path().join("old-version.png").exists());
        assert!(!dir.path().join("orphan.png").exists());
    }

    #[test]
    fn prune_orphans_on_missing_dir_is_a_noop() {
        let conn = crate::db::open_in_memory().unwrap();
        assert_eq!(prune_orphans(&conn, Path::new("/no/existe")).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn save_named_dedupes() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(save_named(dir.path(), "a.png", PNG).unwrap(), ("a.png".into(), true));
        assert_eq!(save_named(dir.path(), "a.png", PNG).unwrap(), ("a.png".into(), false));
        let (other, created) = save_named(dir.path(), "a.png", b"\x89PNG\r\n\x1a\n1111").unwrap();
        assert!(created && other != "a.png");
    }
}
