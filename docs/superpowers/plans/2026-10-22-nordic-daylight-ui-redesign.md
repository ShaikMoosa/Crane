# Crane P0 Nordic Daylight UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Crane's current warm, serif-heavy desktop UI with the approved Nordic Daylight Archive system and add live formatted Markdown inside the existing native `egui::TextEdit`, without changing persisted data or weakening editing, save, accessibility, or offline guarantees.

**Architecture:** Keep `CraneApp` as the orchestration boundary and the existing typed application-command path unchanged. Add semantic theme/layout primitives in `crane-desktop`, a presentation-neutral Markdown span parser in `crane-domain`, and a desktop-only cached `LayoutJob` adapter consumed by the existing stable per-note `TextEdit`. Make the visual replacement in vertical slices so every task has a focused RED/GREEN test and preserves a runnable app.

**Tech Stack:** Rust 1.99.1, egui/eframe 0.37, wgpu 30, egui_kittest 0.37, pulldown-cmark 0.13.4, existing vendored Lucide 1.60 subset, PowerShell release guards, Windows NSIS packaging. No new crate dependency is planned.

**Spec:** `docs/superpowers/specs/2026-10-22-nordic-daylight-ui-redesign-design.md`

## Global Constraints

- Preserve the pure-Rust `egui`/`eframe`/`wgpu` stack. Do not add Tauri, Electron, a webview, JavaScript/TypeScript, Node.js, remote scripts, remote fonts, or required services.
- Preserve schema version 1, note IDs and bodies, JSON/backup compatibility, local-first behavior, atomic saves, transfer locks, and the existing command/query boundary.
- Bear is a quality reference only. Do not copy Bear code, text, assets, branding, themes, screenshots, or pixel geometry.
- Keep native `egui::TextEdit` as the editor. Do not introduce a custom editing engine or a paint-over editor.
- Keep the draft `String` as the only editable and persisted source. Highlighting must never insert, remove, replace, hide, or reorder source characters.
- Preserve the stable `editor_widget_id(note_id, field)`, `TextEditState`, caret/selection, IME, clipboard, undo/redo, completion, formatting, autosave, failed-save retention, and save-before-navigation gate.
- Do not log note text, link targets, imported bodies, backup payloads, or highlighted substrings.
- Use semantic color/type roles in views; production views must not reintroduce the retired warm charcoal/paper literals.
- Keep all rendered icons in the single vendored Lucide subset and update its provenance manifest and notices. Do not retain unused SVGs.
- Do not update `DESIGN.md` or `.impeccable/design.json` until the native finish review returns `ship`; those documents must describe the finished UI, not the proposal.
- Follow test-driven development for each task: observe the focused RED result before production edits, then the focused GREEN result before committing.

## Review Focus

1. **Editing integrity:** hostile UTF-8, CRLF, malformed/incomplete Markdown, IME composition, selection, clipboard, and multi-step undo/redo must remain byte-safe and native. Owned by Tasks 5–6; tests: `crates/domain/tests/markdown_highlight.rs`, `apps/desktop/tests/markdown_layouter.rs`, `apps/desktop/tests/editor.rs`, and physical Windows acceptance.
2. **Cache and large-note behavior:** a body revision parses once; note identity/content changes invalidate; width/theme changes reflow from cached spans; a near-2 MiB note remains editable without knowledge-index or preview recomputation. Owned by Tasks 6 and 8; tests: `apps/desktop/tests/markdown_layouter.rs` and `apps/desktop/tests/editor.rs`.
3. **No draft loss:** navigation, note actions, inspector links, create, export, and transfer entry points still pass through the existing save gate; save failures keep the draft and selection. Owned by Tasks 3–4 and 8; tests: `apps/desktop/tests/editor.rs`, `navigation.rs`, `inspector.rs`, and `transfers.rs`.
4. **Responsive geometry:** wide, collapsed, single-pane, inspector-sheet, docked-inspector, long-line, minimum-size, and 100/150/200% DPI states must not clip or destroy the editor measure. Owned by Tasks 2, 4, and 7; tests: `apps/desktop/tests/navigation.rs`, `inspector.rs`, and native screenshots.
5. **Accessibility and preferences:** visible focus, selected state beyond color, 40/44-point targets, named icon controls, keyboard order, Escape stack, dark mode, high contrast, and reduced motion must remain valid. Owned by Tasks 1, 3–4, 7–9; tests: `apps/desktop/tests/accessibility.rs`, `icons.rs`, `navigation.rs`, plus Narrator/native acceptance.

## File Map

### New files

- `crates/domain/src/highlight.rs` — presentation-neutral Markdown span roles and byte-safe highlighter.
- `crates/domain/tests/markdown_highlight.rs` — exact ranges, malformed input, UTF-8, and span-invariant coverage.
- `apps/desktop/src/markdown_layouter.rs` — span cache, token-to-`TextFormat` mapping, `LayoutJob` validation, and plain-text fallback builder.
- `apps/desktop/tests/theme.rs` — semantic token/type role and contrast relationship tests.
- `apps/desktop/tests/markdown_layouter.rs` — cache invalidation, reflow, source identity, and fallback tests.
- `apps/desktop/assets/icons/lucide/ellipsis.svg`, `settings.svg`, `database-backup.svg` — the only new Lucide assets required by the approved shell.

### Modified files

- `crates/domain/src/lib.rs` — export the highlighter API.
- `apps/desktop/src/lib.rs` — export the desktop layouter module.
- `apps/desktop/src/theme.rs` — Nordic semantic tokens, typography roles, egui visuals, and pane painting.
- `apps/desktop/src/view_state.rs` — responsive metrics, editor measure, inspector presentation, and focus transition helpers.
- `apps/desktop/src/app.rs` — compose the new shell, centered editor, inspector sheet/dock, live layouter state, focus transition, and status/action routing.
- `apps/desktop/src/draft.rs` — expose a read-only save-presentation state without changing save transitions.
- `apps/desktop/src/sidebar.rs` — redesigned library destinations, tag hierarchy, selected marker, and compact rail.
- `apps/desktop/src/views/library.rs` — quiet Data/Settings footer and transfer/status presentation.
- `apps/desktop/src/views/note_list.rs` — compact header, circular create action, flat rows, metadata, tags, and empty results.
- `apps/desktop/src/views/editor.rs` — centered editor chrome, action menu, save state, and native Markdown layouter wiring.
- `apps/desktop/src/views/preview.rs`, `views/restore.rs`, `formatting.rs`, `inspector.rs`, `quick_open.rs`, `knowledge_navigation.rs` — apply the same token, spacing, focus, popup, and sheet vocabulary.
- `apps/desktop/src/icons.rs`, `apps/desktop/assets/icons/lucide/manifest.json`, `THIRD_PARTY_NOTICES.md` — extend and verify the bounded Lucide catalog.
- `apps/desktop/tests/navigation.rs`, `editor.rs`, `formatting.rs`, `icons.rs`, `inspector.rs`, `accessibility.rs`, `quick_open.rs`, `knowledge_navigation.rs`, `restore_dialog.rs`, `transfers.rs` — update intentional labels/geometry and add preservation coverage.
- `docs/migration/windows-acceptance.md` — record only evidence actually performed in the final native pass.
- `DESIGN.md`, `.impeccable/design.json` — regenerate from the shipped interface only after the finish verdict.

`crates/domain/src/model.rs`, `crates/domain/src/backup.rs`, `crates/storage/**`, `fixtures/compat/v1/**`, and the serialized schema are not modified.

---

### Task 1: Establish Nordic semantic tokens and typography

**Files:**
- Create: `apps/desktop/tests/theme.rs`
- Modify: `apps/desktop/src/theme.rs`

**Interfaces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeTokens {
    pub shell_background: Color32,
    pub rail_background: Color32,
    pub list_background: Color32,
    pub editor_background: Color32,
    pub surface_hover: Color32,
    pub surface_selected: Color32,
    pub surface_pressed: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_disabled: Color32,
    pub rule: Color32,
    pub rule_strong: Color32,
    pub focus_ring: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub on_status: Color32,
    pub markdown_marker: Color32,
    pub markdown_heading: Color32,
    pub markdown_emphasis: Color32,
    pub markdown_link: Color32,
    pub markdown_tag: Color32,
    pub markdown_quote: Color32,
    pub markdown_code: Color32,
    pub markdown_code_background: Color32,
}

pub fn tokens(choice: ThemeChoice) -> ThemeTokens;
pub fn install_local_fonts(context: &egui::Context);
pub fn apply(context: &egui::Context, choice: ThemeChoice);
```

The light authority begins near `#F4F5F3` shell, `#E8EAE7` list, `#FFFFFF` editor, `#202124` primary text, and `#D85645` accent. Dark values are cool near-black/graphite equivalents. `Heading` and `Body` use `Crane Interface`; only `Monospace` uses a monospace family. Keep the serif family load only if preview compatibility still needs it during this task, then remove its production use.

- [ ] **Step 1: Write the failing theme tests.** Assert that every semantic field exists; rail/list/editor are distinct in both themes; ordinary text has at least 4.5:1 contrast over its owning background; focus/accent has at least 3:1 non-text contrast; `accent != danger`; light surfaces are cool-neutral rather than the old `#242523/#F1EEE7/#FBFAF7` set; `TextStyle::Body` and `Heading` resolve to `Crane Interface` after `apply`.
- [ ] **Step 2: Run RED.** Run `cargo test -p crane-desktop --test theme --locked`. Expected: compile failure because `ThemeTokens`/`tokens` do not exist and the current font roles are serif.
- [ ] **Step 3: Implement the tokens and type roles.** Make `pane_colors` a temporary adapter over `tokens` so the current UI remains runnable. Apply tokens to egui visuals, selection, disabled/hover/active strokes, popup/window fill, and type styles. Keep fixed role sizes and OS scaling.
- [ ] **Step 4: Run GREEN and nearby regression.** Run `cargo test -p crane-desktop --test theme --test navigation --locked`. Expected: PASS; pre-redesign navigation remains behaviorally intact.
- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/src/theme.rs apps/desktop/tests/theme.rs
git commit -m "feat(ui): add Nordic semantic theme and typography roles"
```

### Task 2: Rebuild responsive shell geometry and centered editor measure

**Files:**
- Modify: `apps/desktop/src/view_state.rs`
- Modify: `apps/desktop/src/theme.rs`
- Modify: `apps/desktop/src/app.rs`
- Modify: `apps/desktop/tests/navigation.rs`
- Modify: `apps/desktop/tests/inspector.rs`

**Interfaces and fixed geometry:**

```rust
pub const RAIL_WIDTH: f32 = 208.0;
pub const COLLAPSED_RAIL_WIDTH: f32 = 64.0;
pub const NOTE_LIST_WIDTH: f32 = 350.0;
pub const COLLAPSED_NOTE_LIST_WIDTH: f32 = 320.0;
pub const EDITOR_MEASURE_MAX: f32 = 700.0;
pub const EDITOR_SIDE_INSET: f32 = 16.0;
pub const INSPECTOR_WIDTH: f32 = 320.0;

pub fn layout_mode(width_points: f32) -> LayoutMode; // Wide >= 1200, CollapsedRail >= 720, otherwise SinglePane
pub fn editor_measure(available_points: f32) -> f32;
pub enum InspectorPresentation { Docked, Sheet }
pub fn inspector_presentation(editor_pane_points: f32) -> InspectorPresentation;
```

`editor_measure` returns `min(700, max(0, available - 32))` and the caller centers that region. The inspector docks only when subtracting `INSPECTOR_WIDTH` and separators still leaves at least 660 points of writing measure; otherwise it is a right-anchored sheet. At the 1440×900 reference viewport the rail is 208, browser 350, and editor measure is 660–700.

- [ ] **Step 1: Rewrite the geometry tests first.** Add table tests for 1440 wide, 1000 collapsed, 640 single-pane; assert the body rect is centered in its editor pane, never exceeds 700 points, keeps at least 44-point compact navigation, and the inspector selects sheet/dock by editor measure rather than a magic total width. Preserve selected note across every mode.
- [ ] **Step 2: Run RED.** Run `cargo test -p crane-desktop --test navigation --test inspector --locked`. Expected: failures on the old 236/330 widths, old `>900/>680` breakpoints, full-width editor, and old inspector threshold.
- [ ] **Step 3: Implement shell metrics and composition.** Move width decisions into `view_state`; have `app.rs` consume them. Center one editor child region instead of stretching the `TextEdit` to the full pane. Keep the existing `MobilePane` transitions and `request_after_save` calls unchanged. Render the sheet after the editor so it layers above without stealing the editor's persisted state.
- [ ] **Step 4: Run GREEN.** Run `cargo test -p crane-desktop --test navigation --test inspector --locked`. Expected: PASS at all tested widths with no horizontal clipping.
- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/src/view_state.rs apps/desktop/src/theme.rs apps/desktop/src/app.rs apps/desktop/tests/navigation.rs apps/desktop/tests/inspector.rs
git commit -m "feat(ui): rebuild Nordic responsive shell geometry"
```

### Task 3: Replace the library rail and note browser

**Files:**
- Create: `apps/desktop/assets/icons/lucide/ellipsis.svg`
- Create: `apps/desktop/assets/icons/lucide/settings.svg`
- Create: `apps/desktop/assets/icons/lucide/database-backup.svg`
- Modify: `apps/desktop/assets/icons/lucide/manifest.json`
- Modify: `apps/desktop/src/icons.rs`
- Modify: `apps/desktop/src/sidebar.rs`
- Modify: `apps/desktop/src/views/library.rs`
- Modify: `apps/desktop/src/views/note_list.rs`
- Modify: `apps/desktop/src/app.rs`
- Modify: `apps/desktop/tests/icons.rs`
- Modify: `apps/desktop/tests/navigation.rs`
- Modify: `apps/desktop/tests/accessibility.rs`
- Modify: `THIRD_PARTY_NOTICES.md`

**Behavior:** The rail shows Crane identity, existing collection/count semantics, nested tags, and bottom Data/Settings controls. The selected destination has a higher-luminance row plus a 3-point accent marker; inactive destinations do not use accent. The browser header owns collection/count, search, Quick Open, and the sole persistent circular accent `New note` button. Rows expose title, a maximum two-line plain excerpt, relative time, and at most two tag/state chips without parsing full Markdown. Render the fixed-height rows with `ScrollArea::show_rows` so a large library only builds visible rows. Empty results name the active filter and expose the relevant recovery action; extend `NoteListAction` with explicit `create` and `clear_search` intents and route `create` through `request_after_save`.

- [ ] **Step 1: Write failing harness tests.** At 1440 points assert the rail and browser labels, collection counts, search, Quick Open, circular `New note`, selected states, bounded metadata, and empty-filter recovery. At 1000 points assert every collapsed destination still has a role/name/tooltip. Add a 1,000-note fixture and assert the accessibility tree contains only the visible row window, then scroll and select a late row. Add a pure relative-time table (`now`, `5m`, `3h`, `2d`, older date) and verify it never underflows for future timestamps. Update accessibility expectations so the visual order is rail → browser header/search → rows → editor.
- [ ] **Step 2: Extend icon provenance tests before assets.** Add `Ellipsis`, `Settings`, and `DatabaseBackup` expectations to `icons.rs`; run `cargo test -p crane-desktop --test icons --test navigation --test accessibility --locked`. Expected: RED because catalog/assets and redesigned controls are absent.
- [ ] **Step 3: Add only the three upstream Lucide 1.60 SVGs.** Record exact source name, version, SHA-256, and Feather-derived flag in `manifest.json`; update `THIRD_PARTY_NOTICES.md` when the manifest marks an asset Feather-derived. No remote references remain in an SVG.
- [ ] **Step 4: Implement flat rows and rail/browser hierarchy.** Use token-driven custom row painting where egui's default selectable button cannot provide the accent marker. Attach `WidgetInfo::selected`, tooltips, and minimum targets explicitly. Continue returning only existing `SidebarIntent`/`NoteListAction` values and route `Create`/selection through the current save gate.
- [ ] **Step 5: Run GREEN.** Run `cargo test -p crane-desktop --test icons --test navigation --test accessibility --locked`. Expected: PASS, with manifest count exactly matching `ALL_ICONS` and no unused SVG.
- [ ] **Step 6: Commit.**

```bash
git add apps/desktop/assets/icons/lucide apps/desktop/src/icons.rs apps/desktop/src/sidebar.rs apps/desktop/src/views/library.rs apps/desktop/src/views/note_list.rs apps/desktop/src/app.rs apps/desktop/tests/icons.rs apps/desktop/tests/navigation.rs apps/desktop/tests/accessibility.rs THIRD_PARTY_NOTICES.md
git commit -m "feat(ui): replace library rail and note browser"
```

### Task 4: Simplify editor chrome, save feedback, actions, and inspector presentation

**Files:**
- Modify: `apps/desktop/src/draft.rs`
- Modify: `apps/desktop/src/views/editor.rs`
- Modify: `apps/desktop/src/inspector.rs`
- Modify: `apps/desktop/src/app.rs`
- Modify: `apps/desktop/tests/drafts.rs`
- Modify: `apps/desktop/tests/editor.rs`
- Modify: `apps/desktop/tests/inspector.rs`
- Modify: `apps/desktop/tests/accessibility.rs`
- Modify: `apps/desktop/tests/transfers.rs`

**Interfaces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePresentation { Saved, Saving, Failed }

impl DraftCoordinator {
    pub fn save_presentation(&self) -> SavePresentation;
}
```

Keep `EditorActions` as the intent boundary. The toolbar permanently shows pane/focus navigation, preview/edit, inspector, quiet save feedback, and a `Note actions` disclosure. Pin/unpin, archive/unarchive, trash/restore, and export move into that menu with their existing accessible names. `Save now` remains available from the menu and `Ctrl+S`, but is no longer visually dominant. A failed save remains inline above the body with the message and `Retry save`.

- [ ] **Step 1: Add RED tests for the state model and hierarchy.** Test `Saved/Save in progress/Failed` mapping without changing coordinator transitions. Harness tests assert primary toolbar controls, `Note actions` disclosure state, all secondary actions after opening, menu target size, destructive naming, tab order, Escape dismissal, editor-focus restoration, transfer-disabled actions, and failed-save retry/draft retention.
- [ ] **Step 2: Run RED.** Run `cargo test -p crane-desktop --test drafts --test editor --test inspector --test accessibility --test transfers --locked`. Expected: failures because the current toolbar exposes every action and has dominant Save/Export buttons.
- [ ] **Step 3: Implement editor presentation only.** Do not change command dispatch. Reuse current `EditorActions` booleans so `app.rs` continues to call `request_after_save`, `begin_transfer`, and draft methods at the same boundaries. Render inspector as the Task 2 sheet/dock result and preserve the current inspector tab across note switches.
- [ ] **Step 4: Run GREEN.** Run the same five test binaries. Expected: PASS, including save-failure retention and inspector link navigation through the gate.
- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/src/draft.rs apps/desktop/src/views/editor.rs apps/desktop/src/inspector.rs apps/desktop/src/app.rs apps/desktop/tests/drafts.rs apps/desktop/tests/editor.rs apps/desktop/tests/inspector.rs apps/desktop/tests/accessibility.rs apps/desktop/tests/transfers.rs
git commit -m "feat(ui): simplify editor actions and inspector chrome"
```

### Task 5: Add presentation-neutral Markdown spans in the domain

**Files:**
- Create: `crates/domain/src/highlight.rs`
- Create: `crates/domain/tests/markdown_highlight.rs`
- Modify: `crates/domain/src/lib.rs`

**Interfaces and invariants:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownSpan {
    pub byte_range: Range<usize>,
    pub role: MarkdownRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownRole {
    Plain,
    Marker,
    Heading(u8),
    Strong,
    Emphasis,
    Strikethrough,
    LinkText,
    LinkDestination,
    WikiLink,
    Tag,
    QuoteMarker,
    ListMarker,
    TaskMarker { checked: bool },
    InlineCode,
    CodeBlock,
}

pub fn highlight_markdown(source: &str) -> Vec<MarkdownSpan>;
```

For non-empty input, spans are ordered, non-overlapping, gap-free, UTF-8-boundary safe, and cover `0..source.len()`. Empty input returns an empty vector. Precedence is `CodeBlock > InlineCode > WikiLink/link destination > task/list/quote/heading markers > strong/emphasis/strike/link text/tag > Plain`. Code suppresses tag/wiki/link interpretation. Existing `knowledge::parse_wiki_links`, `extract_tags` grammar, and pulldown-cmark events/ranges are reused; the highlighter must not create a competing persisted parser.

- [ ] **Step 1: Write table-driven RED tests.** Cover ATX headings 1–6, strong/emphasis/strike, inline and fenced code, links and destinations, wiki links, tags, quote/list/task markers, escaped delimiters, nested roles, CRLF, Devanagari/CJK/combining/emoji, unclosed fences/backticks/brackets/emphasis, empty input, and source immutability. Add one invariant assertion used by every case: full coverage, boundary-safe, adjacent, ordered, non-overlapping.
- [ ] **Step 2: Run RED.** Run `cargo test -p crane-domain --test markdown_highlight --locked`. Expected: compile failure because `highlight_markdown` and role types do not exist.
- [ ] **Step 3: Implement the scanner/composer.** Use byte offsets only at verified char boundaries. Derive coarse semantic ranges from pulldown-cmark `into_offset_iter`, reuse knowledge code-range/wiki helpers where appropriate, scan source delimiters conservatively, then resolve overlaps by the pinned precedence into a complete segment stream. Malformed input falls back to `Marker`/`Plain`; it never panics or drops bytes.
- [ ] **Step 4: Run GREEN and domain regression.** Run `cargo test -p crane-domain --locked`. Expected: PASS for highlighter, preview, knowledge, Markdown import/export, backups, queries, and compatibility fixtures.
- [ ] **Step 5: Commit.**

```bash
git add crates/domain/src/lib.rs crates/domain/src/highlight.rs crates/domain/tests/markdown_highlight.rs
git commit -m "feat(domain): expose byte-safe Markdown highlight spans"
```

### Task 6: Add the cached native TextEdit Markdown layouter

**Files:**
- Create: `apps/desktop/src/markdown_layouter.rs`
- Create: `apps/desktop/tests/markdown_layouter.rs`
- Modify: `apps/desktop/src/lib.rs`
- Modify: `apps/desktop/src/app.rs`
- Modify: `apps/desktop/src/views/editor.rs`
- Modify: `apps/desktop/tests/editor.rs`
- Modify: `apps/desktop/tests/formatting.rs`
- Modify: `apps/desktop/tests/knowledge_navigation.rs`

**Interfaces:**

```rust
#[derive(Default)]
pub struct MarkdownHighlightCache { /* note id + body fingerprint + cached spans + parse count */ }

#[derive(Default)]
pub struct MarkdownLayouter { /* cache plus last safe visible error */ }

impl MarkdownLayouter {
    pub fn layout_job(
        &mut self,
        note_id: &str,
        revision: u64,
        source: &str,
        wrap_width: f32,
        choice: ThemeChoice,
    ) -> Result<egui::text::LayoutJob, MarkdownLayoutError>;
    pub fn parse_count(&self) -> u64;
}

pub fn plain_text_job(source: &str, wrap_width: f32, choice: ThemeChoice) -> LayoutJob;
```

Extend the existing `CacheStats` with `markdown_parses: u64`; `CraneApp::cache_stats()` reports `MarkdownLayouter::parse_count()` so integration tests can prove cache behavior without exposing note content.

The cache key includes note identity plus content length/hash; revision is diagnostic and prevents accidental cross-draft reuse, but content fingerprinting catches the same-frame edit before `DraftCoordinator::mark_edited` advances revision. Theme/width rebuild `LayoutJob` from cached spans without reparsing. `layout_job` validates span coverage before slicing. The TextEdit closure exactly matches egui 0.37: `FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley>` and calls `ui.fonts_mut(|fonts| fonts.layout_job(job))`.

- [ ] **Step 1: Write cache/layout RED tests.** Assert first body parses once; identical frame parses zero additional times; width-only and light/dark reflow do not reparse; changed body and switched note each parse once; `job.text == source`; sections are gap-free and cover the source; code uses monospace/background; heading/link/tag/marker roles use semantic tokens. Inject an invalid span set through a `#[cfg(test)]` helper and assert a plain-text job plus a generic safe error that contains no source text.
- [ ] **Step 2: Run RED.** Run `cargo test -p crane-desktop --test markdown_layouter --locked`. Expected: compile failure because the module/cache do not exist.
- [ ] **Step 3: Implement the layouter and cache.** Use only existing dependencies. Bound heading size changes, set a stable explicit line height, keep trailing whitespace, and keep every Markdown marker visible. Do not persist the cache.
- [ ] **Step 4: Wire the native editor.** Add one `MarkdownLayouter` to `CraneApp`; pass it and the current note identity/revision through `EditorView`; attach `.layouter(&mut closure)` to the existing multiline `TextEdit` without changing its ID, buffer, completion, cursor-jump, or formatting-state code. On layout error, use `plain_text_job` for that frame and surface `Markdown styling unavailable; editing is unchanged.` without logging content.
- [ ] **Step 5: Add preservation tests before GREEN.** Extend editor/formatting/navigation tests for source byte identity after styling, Unicode edit/save/switch, same-note undo, completion insertion, format transform, caret jump, save failure, and preview-cache isolation. Add a near-2 MiB body test asserting one highlight parse and no preview/query recompute; record parse counts rather than brittle wall-clock limits.
- [ ] **Step 6: Run GREEN.** Run `cargo test -p crane-desktop --test markdown_layouter --test editor --test formatting --test knowledge_navigation --locked`. Expected: PASS with the native multiline accessibility role still present.
- [ ] **Step 7: Commit.**

```bash
git add apps/desktop/src/lib.rs apps/desktop/src/markdown_layouter.rs apps/desktop/src/app.rs apps/desktop/src/views/editor.rs apps/desktop/tests/markdown_layouter.rs apps/desktop/tests/editor.rs apps/desktop/tests/formatting.rs apps/desktop/tests/knowledge_navigation.rs
git commit -m "feat(editor): add cached live Markdown layout to native TextEdit"
```

### Task 7: Restyle formatting, inspector, overlays, dialogs, states, dark mode, and focus mode

**Files:**
- Modify: `apps/desktop/src/formatting.rs`
- Modify: `apps/desktop/src/inspector.rs`
- Modify: `apps/desktop/src/quick_open.rs`
- Modify: `apps/desktop/src/knowledge_navigation.rs`
- Modify: `apps/desktop/src/views/preview.rs`
- Modify: `apps/desktop/src/views/restore.rs`
- Modify: `apps/desktop/src/view_state.rs`
- Modify: `apps/desktop/src/app.rs`
- Modify: `apps/desktop/tests/formatting.rs`
- Modify: `apps/desktop/tests/inspector.rs`
- Modify: `apps/desktop/tests/quick_open.rs`
- Modify: `apps/desktop/tests/knowledge_navigation.rs`
- Modify: `apps/desktop/tests/restore_dialog.rs`
- Modify: `apps/desktop/tests/navigation.rs`
- Modify: `apps/desktop/tests/accessibility.rs`

**Behavior:** Every transient surface uses Nordic tokens, modest radii, quiet rules, and elevation only for sheets/menus/overlays. Focus mode uses `Context::animate_bool_with_time(..., 0.18)` to dim surrounding panes and withdraw them; reduced motion uses an immediate 0-second logical transition. The active note, collection/tag/filter, inspector tab/open state, and body focus restore on exit. Dark mode preserves the same three-plane value hierarchy and never returns to warm charcoal/paper.

- [ ] **Step 1: Write RED state and harness tests.** Cover popup/window fills through token helpers, active formatting state beyond color, Quick Open/chooser/restore named controls, Escape priority, sheet focus restoration, dark-pane ordering, focus-mode state restoration, and `focus_transition_seconds(false) == 0.18` / `true == 0.0`. Test both theme choices and 1440/1000/640 layouts.
- [ ] **Step 2: Run RED.** Run the seven affected desktop test binaries. Expected: visual-state assertions fail against default egui surfaces and the current immediate focus hide.
- [ ] **Step 3: Apply tokens and focus transition.** Keep native file/folder dialogs untouched. Use the transition factor only for pane opacity/width presentation; do not animate body text, caret, routine typing, or command state. Keep the existing centralized Escape stack and mutation locks.
- [ ] **Step 4: Run GREEN.** Run `cargo test -p crane-desktop --test formatting --test inspector --test quick_open --test knowledge_navigation --test restore_dialog --test navigation --test accessibility --locked`. Expected: PASS in light/dark and responsive fixtures.
- [ ] **Step 5: Commit.**

```bash
git add apps/desktop/src/formatting.rs apps/desktop/src/inspector.rs apps/desktop/src/quick_open.rs apps/desktop/src/knowledge_navigation.rs apps/desktop/src/views/preview.rs apps/desktop/src/views/restore.rs apps/desktop/src/view_state.rs apps/desktop/src/app.rs apps/desktop/tests/formatting.rs apps/desktop/tests/inspector.rs apps/desktop/tests/quick_open.rs apps/desktop/tests/knowledge_navigation.rs apps/desktop/tests/restore_dialog.rs apps/desktop/tests/navigation.rs apps/desktop/tests/accessibility.rs
git commit -m "feat(ui): finish Nordic surfaces themes and focus mode"
```

### Task 8: Run full automated regression and native release guards

**Files:**
- Modify only if a discovered regression requires a focused fix and a new/updated owning test.

- [ ] **Step 1: Run formatting.** `cargo fmt --all -- --check`. Expected: exit 0. If it fails, run `cargo fmt --all`, inspect the diff, then rerun the check.
- [ ] **Step 2: Run the complete locked workspace suite.** `cargo test --workspace --all-features --locked`. Expected: exit 0, including v1 compatibility, backup/import/export, failed saves, native editor, UI harness, and new highlight suites.
- [ ] **Step 3: Run Clippy and build.** `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` and `cargo build --workspace --all-targets --all-features --locked`. Expected: both exit 0.
- [ ] **Step 4: Run architectural guards.** `powershell -NoProfile -File scripts/check-layers.ps1`; `powershell -NoProfile -File scripts/check-layers.ps1 -SelfTest`; `powershell -NoProfile -File scripts/check-native-release.ps1`; `powershell -NoProfile -File scripts/check-native-release.ps1 -SelfTest`; `powershell -NoProfile -File scripts/smoke-windows.ps1 -SelfTest`. Expected: all exit 0; no Tauri/webview/JS/Node/remote asset or forbidden layer appears.
- [ ] **Step 5: Build the Windows artifact.** Run `powershell -NoProfile -File scripts/build-windows.ps1` and `powershell -NoProfile -File scripts/smoke-windows.ps1 -InstallerPath <produced-installer>`. Expected: native executable and NSIS installer build, installer metadata matches 0.3.0, smoke checks exit 0. Building does not by itself prove visual or interaction acceptance.
- [ ] **Step 6: Diagnose, do not patch blindly.** For any unexpected result, invoke `superpowers:systematic-debugging`, reproduce the root cause, add the smallest failing test, make the fix, rerun the focused test, then rerun all gates affected by the fix.
- [ ] **Step 7: Commit any gate-driven fixes.** If no files changed, do not create an empty commit.

```bash
git add <only owning source and regression tests>
git commit -m "fix(ui): resolve Nordic redesign regression gates"
```

### Task 9: Perform native Windows acceptance, finish review, and shipped-design documentation

**Files:**
- Modify: `docs/migration/windows-acceptance.md`
- Modify after `ship` only: `DESIGN.md`
- Modify after `ship` only: `.impeccable/design.json`
- Create ignored evidence: `.impeccable/review/*`

- [ ] **Step 1: Launch an isolated preview library.** Do not point the review build at the user's real library. Verify the path/profile isolation before launching.
- [ ] **Step 2: Capture native screenshots.** Use the actual egui executable—not the HTML decision sketch—at light/dark × 1440×900, intermediate, and minimum supported size. Repeat representative views at Windows 100%, 150%, and 200% scale. Capture rail/list/editor, long content, empty filter, save failure, action menu, Quick Open, inspector sheet/dock, focus mode, and restore confirmation into ignored `.impeccable/review/` evidence.
- [ ] **Step 3: Exercise native editing.** Verify Latin, Devanagari, Arabic/RTL, CJK IME, combining marks, emoji, clipboard both directions, mouse/keyboard selection, Home/End, word movement, Ctrl+A/C/X/V/Z/Y, multi-step undo/redo, completion, formatting, long lines, and a near-2 MiB note. Save, switch notes, restart, and compare exact content.
- [ ] **Step 4: Exercise accessibility and preferences.** Verify keyboard-only traversal, Narrator names/roles/values/selected state, visible focus, high contrast, light/dark, reduced motion, Escape stack, 40/44-point targets, and inspector focus restoration.
- [ ] **Step 5: Record only observed evidence.** Update `docs/migration/windows-acceptance.md` with date, host, scale, build/installer hash, exact pass/fail result, and any remaining blocker. Do not check a manual item that was not physically performed.
- [ ] **Step 6: Request the required finish review.** Invoke `superpowers:requesting-code-review` with the approved spec, plan, native screenshots, gate outputs, and the five Review Focus items. The reviewer must be fresh to the implementation and return a clear `ship` or `revise` verdict. For `revise`, return to the owning task, add a RED test when automatable, fix, rerun affected gates and native evidence, then request review again.
- [ ] **Step 7: Document only after `ship`.** Regenerate `DESIGN.md` and `.impeccable/design.json` from the implementation. Remove the retired charcoal/warm-paper/serif rules and record Nordic semantic tokens, humanist-sans roles, 208/350/700 geometry, responsive modes, native TextEdit layouter, Lucide provenance, motion/reduced-motion behavior, and current component states. Every shipping raster must record provenance; ignored review screenshots remain evidence, not product assets.
- [ ] **Step 8: Verify documentation and working tree.** Search for stale claims (`Charcoal Rail`, `Warm Paper`, `Page Owns the Serif`) in current design-system sections, confirm historical specs remain clearly historical, rerun `cargo fmt --all -- --check` and the full workspace suite if documentation generation touched source-adjacent files, then inspect `git diff --check` and `git status --short`.
- [ ] **Step 9: Commit final evidence and design truth.**

```bash
git add docs/migration/windows-acceptance.md DESIGN.md .impeccable/design.json
git commit -m "docs: record shipped Nordic Daylight interface"
```

## Completion Definition

The redesign is complete only when all nine tasks are checked, every automated gate is freshly green, native Windows acceptance is recorded honestly, the finish reviewer returns `ship`, `DESIGN.md` and `.impeccable/design.json` match the shipped interface, and no required interaction or data-compatibility behavior regresses. A compiled installer without those conditions is a test artifact, not the accepted release.
