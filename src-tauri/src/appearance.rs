//! Apariencia personalizable: temas y fragmentos CSS del usuario.
//!
//! Viven en la carpeta de datos de la app (dentro del sandbox en Flatpak):
//!   <data>/themes/*.css    — temas del usuario (los incluidos van en el frontend)
//!   <data>/snippets/*.css  — fragmentos que se activan por separado
//!   <data>/appearance.json — qué tema y qué fragmentos están activos
//!
//! El CSS no puede ejecutar código ni tocar las notas: solo cambia el aspecto.
//! Aun así se valida el nombre para que nadie lea fuera de esas carpetas.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Un CSS más grande que esto casi seguro no es un tema (o es un error).
const MAX_CSS_BYTES: u64 = 512 * 1024;
const SETTINGS_FILE: &str = "appearance.json";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CssKind {
    Theme,
    Snippet,
}

impl CssKind {
    fn dir_name(self) -> &'static str {
        match self {
            CssKind::Theme => "themes",
            CssKind::Snippet => "snippets",
        }
    }

    pub fn dir(self, data_dir: &Path) -> PathBuf {
        data_dir.join(self.dir_name())
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    /// `builtin:<id>` (incluido en la app), `user:<nombre>` (archivo sin `.css`)
    /// o `None` para el predeterminado. Sin prefijo se trata como `user:`.
    pub theme: Option<String>,
    pub color_scheme: ColorScheme,
    /// Fragmentos activos, en el orden en que se aplican.
    pub snippets: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub themes: Vec<String>,
    pub snippets: Vec<String>,
    pub settings: AppearanceSettings,
}

const THEMES_README: &str = "Temas de Notas
==============

Cada archivo .css de esta carpeta aparece en Apariencia > Tus temas.
La forma más fácil de empezar: en Apariencia, «Duplicar» un tema incluido.

Variables que puedes redefinir:
  --bg, --surface, --surface-2, --border   fondos y bordes
  --fg, --muted                            texto principal y secundario
  --primary, --on-primary, --primary-soft  color de acento
  --link, --success, --warning, --danger   estados
  --mark, --code-fg, --focus, --shadow
  --font-sans, --font-mono, --radius
  --hl-keyword, --hl-string, --hl-number, --hl-comment, --hl-title, --hl-attr
                                           colores del código resaltado

Modo claro en :root { ... }
Modo oscuro en :root[data-color-scheme=\"dark\"] { ... }
(No uses @media (prefers-color-scheme): la app permite forzar claro u oscuro.)
";

const EXAMPLE_SNIPPET: &str = r#"/* Fragmento de ejemplo: títulos con el color de acento.
 * Los fragmentos se aplican después del tema, así que pueden ajustarlo. */
.cm-live-h1, .cm-live-h2 { color: var(--primary); }
"#;

/// Crea las carpetas. Si no existían, deja dentro una guía o un ejemplo para
/// que el usuario vea el formato sin tener que buscar documentación.
pub fn ensure_dirs(data_dir: &Path) -> AppResult<()> {
    for (kind, name, content) in [
        (CssKind::Theme, "LEEME.txt", THEMES_README),
        (CssKind::Snippet, "Títulos con acento (ejemplo).css", EXAMPLE_SNIPPET),
    ] {
        let dir = kind.dir(data_dir);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            fs::write(dir.join(name), content)?;
        }
    }
    Ok(())
}

/// Nombres (sin `.css`) de los archivos CSS de la carpeta, ordenados.
pub fn list(data_dir: &Path, kind: CssKind) -> AppResult<Vec<String>> {
    let dir = kind.dir(data_dir);
    let mut names = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else { continue };
        if let Some(stem) = file_name.strip_suffix(".css")
            && valid_name(stem)
        {
            names.push(stem.to_string());
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    Ok(names)
}

/// Un nombre que no puede salir de la carpeta: sin separadores ni «..».
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\', '\0']) && name.len() <= 200
}

pub fn read(data_dir: &Path, kind: CssKind, name: &str) -> AppResult<String> {
    if !valid_name(name) {
        return Err(AppError::validation(format!("Nombre de archivo no válido: «{name}»")));
    }
    let path = kind.dir(data_dir).join(format!("{name}.css"));
    let meta = match fs::metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::not_found(format!("No existe «{name}.css»")));
        }
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() {
        return Err(AppError::validation(format!("«{name}.css» no es un archivo")));
    }
    if meta.len() > MAX_CSS_BYTES {
        return Err(AppError::validation(format!("«{name}.css» ocupa más de {} KiB", MAX_CSS_BYTES / 1024)));
    }
    let bytes = fs::read(&path)?;
    String::from_utf8(bytes).map_err(|_| AppError::validation(format!("«{name}.css» no está en UTF-8")))
}

/// Ajustes guardados. Un archivo ausente o dañado no impide arrancar: se usan los predeterminados.
pub fn load_settings(data_dir: &Path) -> AppearanceSettings {
    fs::read(data_dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Escritura atómica: un corte de luz deja el archivo viejo o el nuevo, nunca uno a medias.
pub fn save_settings(data_dir: &Path, settings: &AppearanceSettings) -> AppResult<()> {
    let theme = settings.theme.as_deref().map(|t| t.strip_prefix("builtin:").or(t.strip_prefix("user:")).unwrap_or(t));
    for name in theme.into_iter().chain(settings.snippets.iter().map(String::as_str)) {
        if !valid_name(name) {
            return Err(AppError::validation(format!("Nombre de archivo no válido: «{name}»")));
        }
    }
    let json = serde_json::to_vec_pretty(settings).map_err(|e| AppError::Internal(e.to_string()))?;
    let tmp = data_dir.join(format!("{SETTINGS_FILE}.tmp"));
    fs::write(&tmp, json)?;
    fs::rename(&tmp, data_dir.join(SETTINGS_FILE))?;
    Ok(())
}

/// Guarda `css` como tema del usuario con un nombre libre («Nord», «Nord (2)»…)
/// y devuelve el nombre elegido. Nunca sobrescribe un archivo existente.
pub fn create_theme(data_dir: &Path, name: &str, css: &str) -> AppResult<String> {
    let base = name.trim();
    if !valid_name(base) {
        return Err(AppError::validation(format!("Nombre de tema no válido: «{name}»")));
    }
    if css.len() as u64 > MAX_CSS_BYTES {
        return Err(AppError::validation("El tema es demasiado grande"));
    }
    let dir = CssKind::Theme.dir(data_dir);
    fs::create_dir_all(&dir)?;
    let taken = list(data_dir, CssKind::Theme)?;
    let free = |candidate: &str| !taken.iter().any(|t| t.to_lowercase() == candidate.to_lowercase());
    let chosen = std::iter::once(base.to_string())
        .chain((2..).map(|n| format!("{base} ({n})")))
        .find(|c| free(c))
        .expect("la secuencia de nombres es infinita");
    // create_new: si otro proceso lo creó entretanto, error en vez de pisarlo.
    use std::io::Write;
    fs::OpenOptions::new().write(true).create_new(true).open(dir.join(format!("{chosen}.css")))?.write_all(css.as_bytes())?;
    Ok(chosen)
}

pub fn state(data_dir: &Path) -> AppResult<AppearanceState> {
    Ok(AppearanceState {
        themes: list(data_dir, CssKind::Theme)?,
        snippets: list(data_dir, CssKind::Snippet)?,
        settings: load_settings(data_dir),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_guia_y_ejemplo_solo_la_primera_vez() {
        let dir = tempfile::tempdir().unwrap();
        ensure_dirs(dir.path()).unwrap();
        assert!(CssKind::Theme.dir(dir.path()).join("LEEME.txt").is_file());
        assert!(list(dir.path(), CssKind::Theme).unwrap().is_empty(), "el LEEME no es un tema");
        assert_eq!(list(dir.path(), CssKind::Snippet).unwrap(), vec!["Títulos con acento (ejemplo)"]);
        // Si el usuario borra el ejemplo, no vuelve a aparecer.
        fs::remove_file(CssKind::Snippet.dir(dir.path()).join("Títulos con acento (ejemplo).css")).unwrap();
        ensure_dirs(dir.path()).unwrap();
        assert!(list(dir.path(), CssKind::Snippet).unwrap().is_empty());
    }

    #[test]
    fn duplicar_tema_no_pisa_y_numera() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(create_theme(dir.path(), "Nord", "a{}").unwrap(), "Nord");
        assert_eq!(create_theme(dir.path(), "nord", "b{}").unwrap(), "nord (2)");
        assert_eq!(read(dir.path(), CssKind::Theme, "Nord").unwrap(), "a{}");
        assert!(create_theme(dir.path(), "../fuera", "x").is_err());
    }

    #[test]
    fn lista_solo_css_ordenados() {
        let dir = tempfile::tempdir().unwrap();
        let themes = CssKind::Theme.dir(dir.path());
        fs::create_dir_all(themes.join("carpeta.css")).unwrap();
        for f in ["b.css", "A.css", "nota.txt", ".oculto.css"] {
            fs::write(themes.join(f), "").unwrap();
        }
        assert_eq!(list(dir.path(), CssKind::Theme).unwrap(), vec!["A", "b"]);
    }

    #[test]
    fn no_lee_fuera_de_la_carpeta() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("secreto.css"), "x").unwrap();
        for name in ["../secreto", "..", "a/b", "", ".oculto"] {
            assert!(matches!(read(dir.path(), CssKind::Theme, name), Err(AppError::Validation(_))), "{name}");
        }
    }

    #[test]
    fn lee_y_rechaza_archivos_enormes() {
        let dir = tempfile::tempdir().unwrap();
        let snippets = CssKind::Snippet.dir(dir.path());
        fs::create_dir_all(&snippets).unwrap();
        fs::write(snippets.join("ok.css"), "a{}").unwrap();
        fs::write(snippets.join("grande.css"), vec![b' '; MAX_CSS_BYTES as usize + 1]).unwrap();
        assert_eq!(read(dir.path(), CssKind::Snippet, "ok").unwrap(), "a{}");
        assert!(matches!(read(dir.path(), CssKind::Snippet, "grande"), Err(AppError::Validation(_))));
        assert!(matches!(read(dir.path(), CssKind::Snippet, "falta"), Err(AppError::NotFound(_))));
    }

    #[test]
    fn ajustes_ida_y_vuelta_y_tolerancia_a_archivo_danado() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_settings(dir.path()), AppearanceSettings::default());
        let s = AppearanceSettings { theme: Some("builtin:sepia".into()), color_scheme: ColorScheme::Dark, snippets: vec!["a".into()] };
        save_settings(dir.path(), &s).unwrap();
        assert_eq!(load_settings(dir.path()), s);
        fs::write(dir.path().join(SETTINGS_FILE), "{ roto").unwrap();
        assert_eq!(load_settings(dir.path()), AppearanceSettings::default());
        for theme in ["../x", "user:../x", "builtin:a/b"] {
            let bad = AppearanceSettings { theme: Some(theme.into()), ..Default::default() };
            assert!(save_settings(dir.path(), &bad).is_err(), "{theme}");
        }
        // Ajustes de la versión anterior (sin colorScheme) siguen leyéndose.
        fs::write(dir.path().join(SETTINGS_FILE), r#"{"theme":"Sepia (ejemplo)","snippets":[]}"#).unwrap();
        assert_eq!(load_settings(dir.path()).color_scheme, ColorScheme::System);
    }
}
