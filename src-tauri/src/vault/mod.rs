//! Intercambio con el exterior: exportar/importar un vault Markdown en zip
//! (compatible con Obsidian y con el export de cogni-vault) y migrar
//! directamente una base de datos de cogni-vault.

pub mod export;
pub mod import_legacy;
pub mod import_zip;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use regex::{Captures, Regex};
use rusqlite::Connection;

use crate::error::AppResult;
use crate::repo::folders;
use crate::util::{now, percent_decode, title_key};

/// Nombre válido como archivo/carpeta en Linux y Windows.
pub fn sanitize_segment(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { ' ' } else { c })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_matches(|c: char| c == '.' || c.is_whitespace());
    let out: String = trimmed.chars().take(120).collect();
    if out.is_empty() { fallback.to_string() } else { out }
}

/// Normaliza una fecha del frontmatter o de cogni-vault a RFC 3339 UTC.
pub fn normalize_ts(raw: Option<&str>) -> String {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else { return now() };
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Millis, true);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s.get(..10).unwrap_or(s), "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339_opts(SecondsFormat::Millis, true);
    }
    now()
}

#[derive(Debug, Default)]
pub struct Frontmatter {
    pub fields: HashMap<String, String>,
    pub tags: Vec<String>,
}

impl Frontmatter {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str).filter(|s| !s.is_empty())
    }
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.starts_with('"') {
        if let Ok(s) = serde_json::from_str::<String>(v) {
            return s;
        }
        return v.trim_matches('"').replace("\\\"", "\"");
    }
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        return v[1..v.len() - 1].replace("''", "'");
    }
    v.to_string()
}

/// Frontmatter YAML mínimo (clave: valor y listas de `tags`). Devuelve el
/// cuerpo sin el bloque.
pub fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    let raw = raw.trim_start_matches('\u{feff}');
    let mut fm = Frontmatter::default();
    let Some(rest) = raw.strip_prefix("---").filter(|r| r.starts_with('\n') || r.starts_with("\r\n")) else {
        return (fm, raw.to_string());
    };
    let Some(end) = rest.find("\n---") else { return (fm, raw.to_string()) };
    let yaml = &rest[..end];
    let after = &rest[end + 4..];
    let body = after.split_once('\n').map(|(_, b)| b).unwrap_or("");

    let mut list_key: Option<String> = None;
    for line in yaml.lines() {
        let trimmed = line.trim_end();
        if let Some(item) = trimmed.trim_start().strip_prefix("- ") {
            if list_key.as_deref() == Some("tags") {
                fm.tags.push(unquote(item));
            }
            continue;
        }
        let Some((k, v)) = trimmed.split_once(':') else { continue };
        let key = k.trim().to_string();
        let v = v.trim();
        if v.is_empty() {
            list_key = Some(key);
            continue;
        }
        list_key = None;
        if key == "tags" {
            if v.starts_with('[') {
                match serde_json::from_str::<Vec<String>>(v) {
                    Ok(list) => fm.tags.extend(list),
                    Err(_) => fm.tags.extend(
                        v.trim_matches(|c| c == '[' || c == ']')
                            .split(',')
                            .map(unquote)
                            .filter(|t| !t.is_empty()),
                    ),
                }
            } else {
                fm.tags.extend(v.split(',').map(unquote).filter(|t| !t.is_empty()));
            }
        } else {
            fm.fields.insert(key, unquote(v));
        }
    }
    (fm, body.trim_start_matches(['\r', '\n']).to_string())
}

static MD_IMAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"!\[([^\]\n]*)\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"\n]*")?\s*\)"#).unwrap());
static WIKI_EMBED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!\[\[([^\]|\n]+)(?:\|[^\]\n]*)?\]\]").unwrap());

/// Reescribe `![alt](ref)` y `![[ref]]` a `![alt](attachments/<nombre>)`
/// cuando `resolve(ref)` devuelve el nombre del adjunto guardado.
pub fn rewrite_image_refs(body: &str, mut resolve: impl FnMut(&str) -> Option<String>) -> String {
    let step = MD_IMAGE.replace_all(body, |c: &Captures| {
        let reference = percent_decode(&c[2]);
        match resolve(&reference) {
            Some(name) => format!("![{}]({}{name})", &c[1], crate::attachments::BODY_PREFIX),
            None => c[0].to_string(),
        }
    });
    WIKI_EMBED
        .replace_all(&step, |c: &Captures| match resolve(c[1].trim()) {
            Some(name) => format!("![]({}{name})", crate::attachments::BODY_PREFIX),
            None => c[0].to_string(),
        })
        .into_owned()
}

pub fn basename(reference: &str) -> &str {
    reference.rsplit(['/', '\\']).next().unwrap_or(reference)
}

/// Crea (o reutiliza) la cadena de carpetas `segments` desde la raíz.
pub struct FolderPaths {
    cache: HashMap<Vec<String>, String>,
    pub created: usize,
}

impl FolderPaths {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), created: 0 }
    }

    pub fn ensure(&mut self, conn: &Connection, segments: &[String]) -> AppResult<Option<String>> {
        let mut parent: Option<String> = None;
        for depth in 1..=segments.len() {
            let key = segments[..depth].iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
            if let Some(id) = self.cache.get(&key) {
                parent = Some(id.clone());
                continue;
            }
            let name: String = sanitize_segment(&segments[depth - 1], "Sin nombre").chars().take(150).collect();
            let (folder, created) = folders::find_or_create(conn, &name, parent.as_deref())?;
            if created {
                self.created += 1;
            }
            self.cache.insert(key, folder.id.clone());
            parent = Some(folder.id);
        }
        Ok(parent)
    }
}

/// `true` si ya hay una nota (viva o en papelera) con ese título en esa carpeta.
pub fn note_exists(conn: &Connection, folder_id: Option<&str>, title: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM notes WHERE title_key = ?1 AND ifnull(folder_id, '') = ifnull(?2, ''))",
        rusqlite::params![title_key(title), folder_id],
        |r| r.get(0),
    )?)
}

pub fn note_id_exists(conn: &Connection, id: &str) -> AppResult<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM notes WHERE id = ?1)", [id], |r| r.get(0))?)
}

/// Etiquetas que el export de cogni-vault añadía de forma automática.
pub fn is_noise_tag(tag: &str) -> bool {
    let t = tag.trim().to_lowercase();
    t.is_empty() || t == "nota" || t == "papelera"
}

pub fn dedupe_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    tags.iter()
        .filter(|t| !is_noise_tag(t))
        .filter(|t| seen.insert(t.trim().to_lowercase()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cogni_vault_frontmatter() {
        let raw = "---\ntitle: \"DomainCore\"\noriginal_title: \"Domain \\\"Core\\\"\"\ntags:\n  - Nota\n  - Rust\ndate: 2026-01-05\n---\n\nCuerpo";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.get("original_title"), Some("Domain \"Core\""));
        assert_eq!(fm.tags, ["Nota", "Rust"]);
        assert_eq!(body, "Cuerpo");
        assert_eq!(normalize_ts(fm.get("date")), "2026-01-05T00:00:00.000Z");
    }

    #[test]
    fn parses_inline_tags_and_no_frontmatter() {
        let (fm, _) = parse_frontmatter("---\ntags: [\"a\", \"b\"]\n---\nx");
        assert_eq!(fm.tags, ["a", "b"]);
        let (fm, body) = parse_frontmatter("# Hola\n---\n");
        assert!(fm.fields.is_empty());
        assert_eq!(body, "# Hola\n---\n");
    }

    #[test]
    fn rewrites_known_images_only() {
        let body = "![a](_attachments/x.png) ![[y.png]] ![b](https://e.com/z.png) ![c](/api/v1/images/x.png)";
        let out = rewrite_image_refs(body, |r| (basename(r) == "x.png").then(|| "x.png".to_string()));
        assert_eq!(out, "![a](attachments/x.png) ![[y.png]] ![b](https://e.com/z.png) ![c](attachments/x.png)");
    }

    #[test]
    fn sanitizes_segments() {
        assert_eq!(sanitize_segment("a/b:c?", "x"), "a b c");
        assert_eq!(sanitize_segment("...", "x"), "x");
    }
}
