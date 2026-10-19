# Ember Notes

Ember Notes is an original, Bear-inspired local Markdown notebook built with Tauri 2, Rust, and plain HTML/CSS/JavaScript. It has no Node.js runtime and no cloud account: notes are stored as JSON in the operating system's application-data directory.

## What works

- Create and autosave Markdown notes
- Search titles, bodies, and tags
- Use nested inline tags such as `#work/research`
- Pin, archive, trash, and restore notes
- Toggle a safe Markdown preview
- Switch between light and dark themes
- Use `Ctrl+N` for a new note and `Ctrl+F` for search
- Preview the UI in a browser with a localStorage-backed adapter

## Run as a desktop app on Windows

Install the prerequisites first:

1. Microsoft C++ Build Tools with **Desktop development with C++**.
2. Rust with the MSVC toolchain (`rustup default stable-msvc`).
3. Microsoft Edge WebView2 Runtime (normally already present on Windows 10/11).

Then open PowerShell in this folder and run:

```powershell
cargo install tauri-cli --version "^2.0"
Set-Location .\src-tauri
cargo tauri dev
```

Build an installer with:

```powershell
Set-Location .\src-tauri
cargo tauri build
```

Build artifacts will appear under `src-tauri\target\release\bundle`.

## Preview without Rust

From this folder, serve the static files and open the printed local URL ending in `/ui/`:

```powershell
python -m http.server 4173
```

The browser preview uses localStorage instead of the native JSON backend, but exercises the same interface and note logic.

## Development workflow with Superpowers

The official Superpowers Codex plugin is installed in the current environment. The repository includes [AGENTS.md](./AGENTS.md), a design spec, and an implementation plan organized around its workflow. Begin feature work with `superpowers:using-superpowers`, then use the applicable qualified skills: brainstorming and planning before implementation, TDD for behavior changes, systematic debugging for failures, and verification-before-completion before declaring work done.

Project workflow documents live in `docs/superpowers/`. Superpowers is a development methodology; it is not bundled into the desktop application or required by end users.

