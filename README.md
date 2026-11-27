<p align="center">
  <img src="public/strand-icon.svg" width="96" height="96" alt="Strand logo">
</p>

<h1 align="center">Strand</h1>

Desktop notes app for Linux and macOS, local-first and cloud-free. Live Markdown editor (Obsidian-style), folders, `[[note]]` links, full-text search, image attachments, trash with restore, version history, and zip export/import.

## Features

- **Live Markdown editor**: no Edit/Read modes, floating toolbar on text selection, `/` menu for blocks, tables, [Mermaid](https://mermaid.js.org/), and [KaTeX](https://katex.org/).
- **Folder organization** with drag and drop, inline renaming, and unique names among siblings.
- **`[[note]]` links** with backlinks and wikilink/embed detection.
- **Full-text search** (FTS5 + BM25, title weighted higher, accent-insensitive).
- **Tags**, normalized and independent from folders.
- **Image attachments** (paste or drag), served through a custom protocol (`attachment://`).
- **Trash** for notes and folders, with restore or purge.
- **Version history** with automatic snapshots.
- **Zip export/import**: custom format, compatible with [Obsidian](https://obsidian.md/).
- **Appearance**: light/dark/system mode, built-in themes (Midnight, Mocha, Sage, Lavender, High Contrast), and user-defined CSS themes/snippets.
- **Optimistic concurrency** (OCC): warns if a note changed elsewhere before overwriting.

## Tech stack

| Layer           | Technology                                                                           |
| --------------- | ------------------------------------------------------------------------------------- |
| Desktop shell   | [Tauri 2](https://tauri.app/)                                                        |
| Backend         | Rust (2024 edition), [rusqlite](https://github.com/rusqlite/rusqlite) with embedded SQLite (WAL) |
| Frontend        | [Vue 3](https://vuejs.org/) + TypeScript + [Pinia](https://pinia.vuejs.org/)         |
| Editor          | [CodeMirror 6](https://codemirror.net/)                                             |
| Frontend build  | [Vite](https://vitejs.dev/)                                                          |
| Package manager | [bun](https://bun.sh/)                                                               |

Linux: distributed as Flatpak and `.deb`. macOS: `.dmg`.

## Prerequisites

- **Rust** (2024 edition) via [rustup](https://rustup.rs/).
- **bun** ≥ 1.4.
- System dependencies for Tauri 2: WebKitGTK and build tools on Linux; Xcode Command Line Tools on macOS. See the [official Tauri prerequisites guide](https://tauri.app/start/prerequisites/).

## Installation

Ready-to-use binaries (`.deb` for Linux, `.dmg` for macOS) are available under the repository's [Releases](https://github.com/cgm98/Strand/releases) section.

> On macOS, since the app isn't signed with an Apple Developer certificate, Gatekeeper may flag it as "from an unidentified developer". To open it: right-click the app → Open.

### Build from source

```bash
git clone https://github.com/cgm98/Strand.git
cd Strand
bun install
```

## Usage

### Development mode (desktop app)

```bash
bun run tauri dev
```

Starts Vite on `http://localhost:1420` and opens the Tauri window with hot reload. The webview console is forwarded to this same terminal.

### Frontend only, in the browser

```bash
bun run dev
```

Useful for fast UI iteration, but without access to Tauri commands (Rust backend).

### Build for production

```bash
bun run tauri build
```

Generates binaries/installers for the host system (`.deb`/`AppImage` on Linux, `.dmg`/`.app` on macOS) in `src-tauri/target/release/bundle/`.

### Backend tests and lint (Rust)

```bash
cd src-tauri
cargo test
cargo clippy --all-targets
```

### Frontend tests and lint

```bash
bun test             # bun test + jsdom
bun run lint          # ESLint
bun run format:check  # Prettier (check only; use `bun run format` to rewrite)
```

## Project structure

```
Strand/
├── src/                  # Frontend: Vue 3 + TypeScript
│   ├── components/       # Editor, sidebar, dialogs, menus
│   ├── composables/      # Reusable logic (autosave, preferences, etc.)
│   ├── stores/           # Global state (Pinia)
│   ├── lib/              # Markdown, sanitization, Tauri API client
│   ├── themes/           # Appearance themes
│   └── test/             # jsdom environment for `bun test` (see bunfig.toml)
├── src-tauri/            # Backend: Rust
│   ├── src/
│   │   ├── commands.rs   # Commands invokable from the frontend
│   │   ├── db.rs         # SQLite setup, migrations
│   │   ├── repo/         # Data access (notes, folders, tags, search, trash)
│   │   ├── vault/        # Export/import and Obsidian integration
│   │   └── attachments.rs
│   ├── capabilities/     # Tauri permissions
│   └── packaging/        # .desktop and metainfo.xml (for the Flatpak manifest)
├── .github/workflows/    # CI: cargo test/clippy + bun test/lint/format
└── scripts/              # Color theme generation and auditing
```

## License

[MIT](./LICENSE)
