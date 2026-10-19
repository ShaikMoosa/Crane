# Crane

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

Rust + Tauri v2 with a static HTML/CSS/JavaScript frontend. The application requires no Node.js runtime or frontend bundler. Node may be used only for development-time tests.

## Users

Primary user: one person writing and organizing private notes on a Windows desktop, often offline. This is inferred from the request for a local Bear-style application and remains easy to revise.

## Product Purpose

Crane is a fast, local-first writing space for capturing Markdown notes, organizing them with inline tags, and finding them again without an account or cloud dependency. Success means opening the app and moving from thought to writing in seconds.

## Positioning

The application combines a calm three-pane writing environment with transparent local storage and a small native Rust backend. It is inspired by Bear's emphasis on Markdown, tags, and visual quiet, but it is an original cross-platform product rather than a copy of Bear's code, branding, assets, or proprietary services.

## Operating Context

- Create and edit notes continuously with automatic local saves.
- Use `#tags` inside note bodies; browse and filter by those tags.
- Search titles and note content.
- Pin important notes, archive inactive notes, and move unwanted notes to trash.
- Import UTF-8 Markdown, export individual notes or the usable library, and create or restore complete Crane backups from the Data menu at the bottom of the library rail.
- Work offline. Notes persist under the operating system's application-data directory.

## Capabilities and Constraints

- MVP: note CRUD, search, inline tag extraction, pinning, archive, trash and restore, Markdown-oriented editor, reading preview, Markdown import/export, complete backup/restore, keyboard shortcuts, responsive two/three-pane layout, light/dark themes, and seeded onboarding notes.
- Native persistence is a JSON document written atomically by Rust. The model is intentionally small enough to migrate later to SQLite or individual Markdown files.
- Markdown import accepts UTF-8 `.md` files up to the documented note and batch limits. Library export writes active notes under `Notes/`, archived notes under `Archive/`, and excludes trash; single-note export can explicitly export any selected note.
- Complete `.crane-backup.json` backups preserve active, archived, and trashed notes with all metadata. Restore replaces the library only after validation, explicit confirmation, and an automatic pre-restore safety backup.
- No sync, accounts, collaboration, encryption, attachments, OCR, PDF/document export, cloud destinations, or scheduled backups in the MVP.
- No network access is required at runtime.
- All frontend-to-native access uses narrow Tauri commands.

## Brand Commitments

The product name is Crane. Its voice is calm, literate, direct, and private. Bear is a functional and interaction reference, not a branding source.

## Evidence on Hand

- User supplied Bear's public site as product reference.
- Bear's public product page emphasizes Markdown, flexible tags, offline/native speed, and a minimal interface.
- No user-owned logo, font, screenshots, or sample corpus was supplied; do not invent claims about adoption, privacy certification, or performance benchmarks.

## Product Principles

1. Writing is always the primary action.
2. Organization emerges from the note through tags rather than setup-heavy folders.
3. Local data is visible, recoverable, and never silently discarded.
4. Power stays close at hand but visually quiet.
5. Every action works from the keyboard as well as the pointer.

## Accessibility & Inclusion

Target WCAG 2.2 AA contrast, visible keyboard focus, semantic controls, reduced-motion support, and layouts usable from compact windows through wide desktop screens.

