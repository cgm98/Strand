use std::collections::BTreeMap;
use std::sync::LazyLock;

use chrono::{SecondsFormat, Utc};
use regex::Regex;

/// Marcas que `search` inserta alrededor de cada coincidencia del snippet.
/// Son caracteres de uso privado (no HTML) para que el frontend escape el
/// texto primero y resalte después, sin riesgo de inyección.
pub const HIGHLIGHT_START: char = '\u{E000}';
pub const HIGHLIGHT_END: char = '\u{E001}';

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Clave laxa para resolver `[[enlaces]]`: minúsculas y solo alfanuméricos.
/// "Domain Core", "domain-core" y "DomainCore" comparten clave.
pub fn title_key(title: &str) -> String {
    title
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

pub fn validate_title(title: &str) -> crate::error::AppResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(crate::error::AppError::validation("El título no puede estar vacío"));
    }
    if t.chars().count() > 300 {
        return Err(crate::error::AppError::validation(
            "El título no puede superar 300 caracteres",
        ));
    }
    if title_key(t).is_empty() {
        return Err(crate::error::AppError::validation(
            "El título debe contener al menos una letra o número",
        ));
    }
    Ok(t.to_string())
}

pub fn validate_folder_name(name: &str) -> crate::error::AppResult<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(crate::error::AppError::validation(
            "El nombre de la carpeta no puede estar vacío",
        ));
    }
    if n.chars().count() > 150 {
        return Err(crate::error::AppError::validation(
            "El nombre de la carpeta no puede superar 150 caracteres",
        ));
    }
    if n.contains('/') || n.contains('\\') {
        return Err(crate::error::AppError::validation(
            "El nombre de la carpeta no puede contener '/' ni '\\'",
        ));
    }
    Ok(n.to_string())
}

static FENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?ms)^[ \t]*(```|~~~).*?^[ \t]*(```|~~~)[ \t]*$").unwrap());
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`[^`\n]*`").unwrap());
static WIKILINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(!?)\[\[([^\[\]|#\n]+)(?:#[^\[\]|\n]*)?(?:\|[^\[\]\n]*)?\]\]").unwrap());

/// Extrae los `[[destinos]]` del Markdown, ignorando bloques de código y
/// embebidos `![[imagen.png]]`. Devuelve `clave -> título tal como se escribió`.
pub fn extract_wikilinks(body: &str) -> BTreeMap<String, String> {
    let without_fences = FENCE.replace_all(body, "");
    let text = INLINE_CODE.replace_all(&without_fences, "");
    let mut out = BTreeMap::new();
    for cap in WIKILINK.captures_iter(&text) {
        if &cap[1] == "!" {
            continue;
        }
        let title = cap[2].trim();
        let key = title_key(title);
        if !key.is_empty() {
            out.entry(key).or_insert_with(|| title.to_string());
        }
    }
    out
}

/// Convierte texto libre en una consulta FTS5 segura: cada palabra va entre
/// comillas (sin operadores) y la última admite prefijo para buscar mientras
/// se escribe.
pub fn fts_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| t.chars().any(char::is_alphanumeric))
        .collect();
    let last = tokens.len().checked_sub(1)?;
    Some(
        tokens
            .iter()
            .enumerate()
            .map(|(i, t)| if i == last { format!("\"{t}\"*") } else { format!("\"{t}\"") })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Decodifica secuencias `%XX` de una URL (p. ej. `mi%20foto.png` → `mi foto.png`).
/// Un `%` que no va seguido de dos dígitos hexadecimales se deja tal cual, y los
/// bytes resultantes que no sean UTF-8 válido se sustituyen por U+FFFD.
pub fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_cases() {
        assert_eq!(percent_decode("a%20b.png"), "a b.png");
        assert_eq!(percent_decode("a%C3%B1o.png"), "año.png");
        assert_eq!(percent_decode("abc%2"), "abc%2");
        assert_eq!(percent_decode("100%zz"), "100%zz");
        assert_eq!(percent_decode("%FF"), "\u{FFFD}");
    }

    #[test]
    fn title_key_ignores_case_spaces_and_symbols() {
        assert_eq!(title_key("Domain Core"), "domaincore");
        assert_eq!(title_key("domain-core"), "domaincore");
        assert_eq!(title_key("Álgebra Lineal"), "álgebralineal");
    }

    #[test]
    fn wikilinks_skip_code_and_embeds() {
        let body = "Ver [[Rust]] y [[Tauri|alias]] y [[Rust#sección]].\n\
                    ![[foto.png]]\n`[[inline]]`\n```bash\n[[ -f x ]]\n```\n";
        let links = extract_wikilinks(body);
        let keys: Vec<_> = links.keys().cloned().collect();
        assert_eq!(keys, vec!["rust", "tauri"]);
    }

    #[test]
    fn fts_query_quotes_tokens_and_prefixes_last() {
        assert_eq!(fts_query("hola mun").as_deref(), Some("\"hola\" \"mun\"*"));
        assert_eq!(fts_query("a\"b OR"), Some("\"ab\" \"OR\"*".into()));
        assert_eq!(fts_query("  ** "), None);
    }
}
