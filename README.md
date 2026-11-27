<p align="center">
  <img src="public/strand-icon.svg" width="96" height="96" alt="Logo de Strand">
</p>

<h1 align="center">Strand</h1>

Aplicación de notas de escritorio para Linux y macOS, local y sin cuentas en la nube. Editor Markdown en vivo (al estilo Obsidian), carpetas, enlaces `[[nota]]`, búsqueda de texto completo, adjuntos de imágenes, papelera con restauración, historial de versiones y export/import en zip.

## Características

- **Editor Markdown en vivo**: sin modos Editar/Leer, barra flotante al seleccionar texto, menú `/` con bloques, tablas, [Mermaid](https://mermaid.js.org/) y [KaTeX](https://katex.org/).
- **Organización en carpetas** con arrastrar y soltar, renombrado en línea y nombres únicos entre hermanos.
- **Enlaces `[[nota]]`** con backlinks y detección de wikilinks/embeds.
- **Búsqueda de texto completo** (FTS5 + BM25, el título pesa más, sin distinguir tildes).
- **Etiquetas** normalizadas, independientes de las carpetas.
- **Adjuntos de imágenes** (pegar o arrastrar), servidos por un protocolo propio (`attachment://`).
- **Papelera** para notas y carpetas, con restaurar o purgar.
- **Historial de versiones** con snapshots automáticos.
- **Export/import en zip**: formato propio, compatible con [Obsidian](https://obsidian.md/).
- **Apariencia**: modo claro/oscuro/sistema, temas incluidos (Medianoche, Moca, Salvia, Lavanda, Alto contraste) y temas/fragmentos CSS propios del usuario.
- **Concurrencia optimista** (OCC): avisa si una nota cambió en otro lado antes de sobrescribir.

## Stack técnico

| Capa                | Tecnología                                                                                      |
| ------------------- | ----------------------------------------------------------------------------------------------- |
| Shell de escritorio | [Tauri 2](https://tauri.app/)                                                                   |
| Backend             | Rust (edición 2024), [rusqlite](https://github.com/rusqlite/rusqlite) con SQLite embebido (WAL) |
| Frontend            | [Vue 3](https://vuejs.org/) + TypeScript + [Pinia](https://pinia.vuejs.org/)                    |
| Editor              | [CodeMirror 6](https://codemirror.net/)                                                         |
| Build frontend      | [Vite](https://vitejs.dev/)                                                                     |
| Gestor de paquetes  | [bun](https://bun.sh/)                                                                          |

Linux: distribuido como Flatpak y `.deb`. macOS: `.dmg`.

## Requisitos previos

- **Rust** (edición 2024) vía [rustup](https://rustup.rs/).
- **bun** ≥ 1.4.
- Dependencias del sistema para Tauri 2: en Linux, WebKitGTK y herramientas de build; en macOS, Xcode Command Line Tools. Ver la [guía oficial de prerrequisitos de Tauri](https://tauri.app/start/prerequisites/).

## Instalación

Binarios listos para usar (`.deb` para Linux, `.dmg` para macOS) en la sección [Releases](https://github.com/cgm98/Strand/releases) del repositorio.

> En macOS, al no estar firmada con un certificado de Apple Developer, Gatekeeper puede marcarla como "de desarrollador no identificado". Para abrirla: clic derecho sobre la app → Abrir.

### Compilar desde el código fuente

```bash
git clone https://github.com/cgm98/Strand.git
cd Strand
bun install
```

## Uso

### Modo desarrollo (app de escritorio)

```bash
bun run tauri dev
```

Arranca Vite en `http://localhost:1420` y abre la ventana de Tauri con recarga en caliente. La consola del webview se reenvía a esta misma terminal.

### Solo el frontend en el navegador

```bash
bun run dev
```

Útil para iterar rápido en la UI, pero sin acceso a los comandos de Tauri (backend Rust).

### Compilar para producción

```bash
bun run tauri build
```

Genera los binarios/instaladores para el sistema donde se ejecuta (`.deb`/`AppImage` en Linux, `.dmg`/`.app` en macOS) en `src-tauri/target/release/bundle/`.

### Tests y lint del backend (Rust)

```bash
cd src-tauri
cargo test
cargo clippy --all-targets
```

### Tests y lint del frontend

```bash
bun test             # bun test + jsdom
bun run lint          # ESLint
bun run format:check  # Prettier (solo verifica; usar `bun run format` para reescribir)
```

## Estructura del proyecto

```
Strand/
├── src/                  # Frontend: Vue 3 + TypeScript
│   ├── components/       # Editor, sidebar, diálogos, menús
│   ├── composables/      # Lógica reutilizable (autoguardado, preferencias, etc.)
│   ├── stores/           # Estado global (Pinia)
│   ├── lib/              # Markdown, sanitizado, cliente de la API de Tauri
│   ├── themes/           # Temas de apariencia
│   └── test/             # Entorno jsdom para `bun test` (ver bunfig.toml)
├── src-tauri/            # Backend: Rust
│   ├── src/
│   │   ├── commands.rs   # Comandos invocables desde el frontend
│   │   ├── db.rs         # Apertura de SQLite, migraciones
│   │   ├── repo/         # Acceso a datos (notas, carpetas, etiquetas, búsqueda, papelera)
│   │   ├── vault/        # Export/import e integración con Obsidian
│   │   └── attachments.rs
│   ├── capabilities/     # Permisos de Tauri
│   └── packaging/        # .desktop y metainfo.xml (para el manifest de Flatpak)
├── .github/workflows/    # CI: cargo test/clippy + bun test/lint/format
└── scripts/              # Generación y auditoría de temas de color
```

## Licencia

[MIT](./LICENSE)
