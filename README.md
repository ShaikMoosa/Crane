# Crane

Crane is an original, Bear-inspired local Markdown notebook built in pure Rust with an egui/eframe native interface and wgpu rendering. It has no webview, no Node.js runtime, and no cloud account: notes are stored as JSON in the operating system's application-data directory.

## What works

- Create and autosave Markdown notes
- Search titles, bodies, and tags
- Use nested inline tags such as `#work/research`
- Pin, archive, trash, and restore notes
- Toggle a safe Markdown preview
- Import one or more UTF-8 `.md` files
- Export the selected note or the active/archived library as ordinary Markdown
- Create and restore complete `.crane-backup.json` backups, including archived and trashed notes
- Switch between light and dark themes
- Use `Ctrl+N` for a new note and `Ctrl+F` for search

## Run on Windows

Prerequisite: Rust with the MSVC toolchain (the repository pins Rust 1.99.1 in `rust-toolchain.toml`). No WebView2 or Node.js is required.

```powershell
cargo run -p crane-desktop
```

By default this opens the production library with the native startup safety path (legacy-process check, pre-migration backup, exclusive library guard). For development against disposable data, use an explicit isolated preview root instead:

```powershell
cargo run -p crane-desktop -- --preview-data-dir artifacts/test-data/dev-preview
```

## Build the Windows installer

```powershell
powershell -NoProfile -File scripts/build-windows.ps1
```

This runs the dependency-layer check, builds the locked release `crane.exe`, verifies it with `scripts/smoke-windows.ps1`, and — when NSIS (`makensis`) is available — produces `artifacts/windows/Crane_0.2.0_x64-setup.exe` with SHA-256 and build provenance. The installer is per-user, preserves `%APPDATA%\app.crane.desktop` data on install/upgrade/uninstall, and replaces a legacy 0.1.0 installation's registration. It is unsigned unless signing credentials are supplied.

## Move and protect your notes

Open **Data** at the bottom of the library rail to import Markdown, export the library, create a complete backup, or inspect a backup for restore. The selected note's action menu also contains **Export note**.

- **Import Markdown** accepts one or more UTF-8 `.md` files. Imports create new active notes and never overwrite existing notes. If any selected file is invalid, none are imported.
- **Export note** writes the selected active, archived, or trashed note as a readable `.md` file with its title as the first level-one heading.
- **Export Library** creates a timestamped folder. Active notes go in `Notes/`, archived notes go in `Archive/`, and trashed notes are intentionally excluded.
- **Create Backup** writes a versioned `.crane-backup.json` file containing every note and its metadata, including archive and trash state.
- **Restore Backup** validates the selected backup and shows its date and note counts before asking for confirmation. Restore replaces the current library; Crane first writes an automatic `pre-restore-*.crane-backup.json` safety backup under its application-data `backups` directory.

All transfers run locally and offline through native file and folder dialogs. Crane does not upload notes or expose general filesystem access. Cancelling a dialog changes nothing.

Data created with the old browser preview can be moved once by hand: see `docs/migration/browser-preview-export.md`. Rollback to the legacy 0.1.0 app uses the retained installer in `artifacts/legacy/` (kept outside the build directories with its SHA-256; the library format is unchanged).

## Development workflow with Superpowers

The official Superpowers Codex plugin is installed in the current environment. The repository includes [AGENTS.md](./AGENTS.md), a design spec, and an implementation plan organized around its workflow. Begin feature work with `superpowers:using-superpowers`, then use the applicable qualified skills: brainstorming and planning before implementation, TDD for behavior changes, systematic debugging for failures, and verification-before-completion before declaring work done.

Project workflow documents live in `docs/superpowers/`. Superpowers is a development methodology; it is not bundled into the desktop application or required by end users.
