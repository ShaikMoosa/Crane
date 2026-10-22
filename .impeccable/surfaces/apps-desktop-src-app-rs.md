---
version: 1
slug: "apps-desktop-src-app-rs"
primary_target: "apps/desktop/src/app.rs"
related_targets: ["apps/desktop/src/theme.rs","apps/desktop/src/views/editor.rs","apps/desktop/src/views/note_list.rs","apps/desktop/src/views/library.rs"]
---

# Crane desktop workspace redesign

Scope: the complete native Crane desktop workspace, including the library rail, note browser, editor, inspector, overlays, menus, responsive modes, light theme, and dark theme.

Mode: Operate.

Audience and job: one person writing and organizing private Markdown notes on Windows, often offline. The primary task is to move from opening Crane to writing, finding, linking, and organizing a note with minimal visual interruption.

Constraints: pure Rust with egui/eframe and wgpu; no webview or Node.js; preserve local-first storage, commands, autosave, backup/restore, import/export, keyboard behavior, accessibility, IME, clipboard, undo/redo, and existing data compatibility. Bear is an interaction and craft reference, not a source of code, branding, assets, or exact visual reproduction.

Chosen direction: Nordic Daylight Archive. The approved decision sketch is `.impeccable/mocks/decision/nordic-daylight.html`. Live formatted Markdown uses native `egui::TextEdit` with a cached Markdown layouter; syntax remains editable and the persisted body remains ordinary Markdown.

## Direction contract

THESIS: Crane is a luminous, nearly silent writing workspace where navigation recedes and the current note receives the light. It refuses the current dark-rail, warm-paper, serif-heavy arrangement and the generic gray productivity shell with a decorative accent.

OWN-WORLD: Cool daylight neutrals form three contiguous planes: pale library rail, soft-gray note browser, and bright-white editor. Humanist sans typography carries both chrome and writing. Crane red is reserved for creation, selection, links, tags, focus, errors, and save confirmation. Controls are compact native rectangles or circles with quiet rules, not floating cards.

STORY: The writer opens directly into a legible library, identifies the current collection and note, and begins typing without entering a separate mode. Structure emerges live from Markdown while navigation, backlinks, formatting, state changes, and data tools remain discoverable but visually secondary.

FIRST VIEWPORT: At 1440×900, a roughly 208-point pale rail contains Crane, collections, nested tags, Data, and Settings; a roughly 350-point soft-gray browser contains collection title, search, new-note action, and flat note rows; the remaining white editor holds a minimal toolbar and a centered 660–700-point writing measure. The active note is the brightest surface. Focus mode expands the editor and briefly dims the surrounding panes before they withdraw.

FORM: Nordic Daylight Archive, fifth in the ordered grounded direction list. `seed:8cc30c5f20d82084`. Its signature move is a focus transition in which the editor gains luminance while library chrome recedes; live Markdown simultaneously strengthens the current structure and subdues editable syntax markers.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

Unresolved decisions: none. P0 includes the full visual replacement, live formatted Markdown, both themes, responsive layouts, accessibility preservation, and native Windows visual verification. It does not add accounts, sync, attachments, encryption, or new persistence formats.
