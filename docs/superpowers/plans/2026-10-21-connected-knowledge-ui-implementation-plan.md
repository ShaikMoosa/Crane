# Crane 0.3 Connected Knowledge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Execution status (2026-10-22):** Tasks 1–14 are implemented. Task 15's compatibility suite, large-library diagnostics, automated repository gates, and installer upgrade/rollback cycle are complete. Physical DPI, live IME, clipboard, extended undo/redo, Narrator, and full keyboard-only acceptance remain open and are tracked in `docs/migration/windows-acceptance.md`; Crane 0.3 is not yet claimed release-ready.

The unchecked boxes below preserve the original implementation recipe and expected RED/GREEN sequence; they are not the current status ledger. The authoritative completion record is the repository history, automated suites, and Windows acceptance document.

**Goal:** Add a memory-only derived knowledge layer and calmer native navigation to Crane — wiki links, backlinks, Quick Open, hierarchical tags, inspector, and formatting — without changing schema version 1 or any persisted library/backup byte.

**Architecture:** Pure domain parsing in `crates/domain/src/knowledge/` turns each committed `Note` into `NoteFacts`; `KnowledgeIndex` in the same crate owns cross-note resolution. `crane-application` owns the index inside `Library`, updates it on the serialized worker only after atomic commit succeeds, and answers typed `Query` values with revision-stamped read models. `crane-desktop` renders those read models through focused controllers (`sidebar`, `quick_open`, `knowledge_navigation`, `inspector`, `formatting`, `icons`) coordinated by `app.rs`; drafts stay desktop-owned and never publish backlinks.

**Tech Stack:** Rust 1.99.1 (pinned), egui/eframe 0.37.0, wgpu 30.0.1 (DX12 on Windows), pulldown-cmark 0.13.4, egui_kittest 0.37.0, vendored Lucide SVG subset rendered through `egui_extras` 0.37.0 `svg` loader. No new domain dependencies.

**Spec:** `docs/superpowers/specs/2026-10-21-connected-knowledge-ui-design.md`

## Global Constraints

- Persisted schema stays `SCHEMA_VERSION = 1`; `Note`/`Store` fields, version-1 JSON fixtures, and `.crane-backup.json` format/version are byte-compatible. The knowledge index is memory-only and is never serialized into the library, backup, settings, or any file.
- Stack stays pure Rust egui/eframe + wgpu. No Tauri, Electron, webview, JavaScript/TypeScript UI, Node.js, remote scripts/fonts/icons, accounts, sync, billing, attachments, OCR, encryption, or storage migration.
- Domain (`crane-domain`) gains no storage, OS, egui, network, or filesystem dependency; it keeps only `pulldown-cmark`, `serde`, `serde_json`. Application (`crane-application`) depends only on `crane-domain`. Enforced by `scripts/check-layers.ps1`.
- Desktop dispatches only typed `Command`/`Query` values; no arbitrary filesystem or shell primitives. Every mutation validates a candidate store, persists through temporary-sibling + rename, and publishes committed state only after the write succeeds. A failed save retains the draft, shows Retry, and blocks note switching/close.
- Markdown bodies remain the source of truth. Renaming a note never rewrites bodies, never merges duplicate titles, and never creates persisted alias metadata. Ambiguous titles are never auto-picked.
- Knowledge parsing is total over arbitrary UTF-8: malformed wiki syntax, unclosed fences, and unmatched brackets are ordinary text, never a save error or panic. Source text is never mutated by parsing.
- Icons are vendored Lucide SVGs under `apps/desktop/assets/icons/lucide/`, embedded with `include_bytes!`, rendered offline, and listed in `apps/desktop/assets/icons/lucide/manifest.json`. Only icons referenced by a shipped control are committed. `THIRD_PARTY_NOTICES.md` carries the full ISC text and the Feather-derived MIT subset with icon names. No icon is fetched at build or runtime.
- Existing shortcuts `Ctrl+N`, `Ctrl+S`, `Ctrl+P`, `Ctrl+F` keep working; 0.3 adds `Ctrl+O` and the Escape stack. Icon-only controls are real buttons with AccessKit names, tooltips, visible focus, 38pt desktop / 44pt compact targets. No Bear text, labels, icon drawings, layout measurements, or assets are copied.
- Every task follows TDD: write the failing test, run it and observe the expected failure, implement minimally, re-run to pass, then commit only that task's files. Unexpected results use `superpowers:systematic-debugging` before code changes.

## Review Focus

- Wiki syntax inside inline code, fenced/indented code, CRLF bodies, and unclosed fences must stay ordinary text with correct UTF-8 byte ranges and no panic — pinned in Task 1.
- Notes whose titles differ only by case/whitespace, empty titles, and literal headings colliding with duplicate suffixes (`Intro` ×2 plus a literal `Intro-2`) must resolve deterministically and never auto-pick — pinned in Tasks 1, 4, and 6.
- Unsaved draft text must appear in Statistics/Outline but never in Backlinks, and a failed save must keep Quick Open/completion/inspector navigation from discarding it — pinned in Tasks 6, 10, 11, and 12.
- IME composition, clipboard, and undo/redo on Windows must survive completion insertion and formatting edits with caret intact — pinned in Tasks 11, 13, and 15.
- Stale Quick Open/completion responses, a 5,000-note library, and a near-2 MiB body must stay bounded, ignore out-of-date results, and never trigger full reparsing per keystroke — pinned in Tasks 6, 10, and 15.

---

## File Structure

### Created

- `crates/domain/src/knowledge/mod.rs` — knowledge type exports and shared normalization helpers.
- `crates/domain/src/knowledge/parser.rs` — per-note scanner: headings, wiki links, byte ranges, code exclusion.
- `crates/domain/src/knowledge/statistics.rs` — pulldown-cmark counts: characters, words, paragraphs, headings, tasks, reading time.
- `crates/domain/src/knowledge/tags.rs` — hierarchical tag projection with distinct-note counts.
- `crates/domain/src/knowledge/index.rs` — `KnowledgeIndex`, resolution, backlinks, Quick Open search corpus.
- `crates/domain/tests/knowledge_parser.rs`, `crates/domain/tests/knowledge_statistics.rs`, `crates/domain/tests/knowledge_tags.rs`, `crates/domain/tests/knowledge_index.rs` — domain TDD suites.
- `crates/application/src/knowledge.rs` — `Query`, `QueryData`, `CollectionView`, and all read-model types.
- `crates/application/tests/knowledge.rs` — index lifecycle, ranking, stale/failure boundary suites. `crates/application/tests/support/mod.rs` is reused unchanged.
- `apps/desktop/src/icons.rs` — bounded Lucide catalog, embedded bytes, accessible icon-button helper.
- `apps/desktop/src/sidebar.rs` — collections, hierarchical tags, fold state, responsive rail.
- `apps/desktop/src/quick_open.rs` — Quick Open overlay state, keyboard, focus restoration.
- `apps/desktop/src/knowledge_navigation.rs` — completion, link activation, ambiguity chooser, heading navigation intents.
- `apps/desktop/src/inspector.rs` — Statistics, Outline, Links, Backlinks tabs; dock/float presentation.
- `apps/desktop/src/formatting.rs` — pure selection-aware Markdown transformations plus toolbar presentation.
- `apps/desktop/assets/icons/lucide/*.svg` and `apps/desktop/assets/icons/lucide/manifest.json` — vendored subset and provenance (exact icon list fixed in Task 7).
- `apps/desktop/tests/quick_open.rs`, `apps/desktop/tests/knowledge_navigation.rs`, `apps/desktop/tests/inspector.rs`, `apps/desktop/tests/formatting.rs`, `apps/desktop/tests/icons.rs` — desktop TDD suites reusing `apps/desktop/tests/common/mod.rs`.
- `THIRD_PARTY_NOTICES.md` — Lucide ISC + Feather-derived MIT notices.

### Modified

- `crates/domain/src/lib.rs` — export the `knowledge` module; no persisted-model change.
- `crates/domain/src/preview.rs` — add `Inline::WikiLink` and split wiki syntax out of text outside code; excerpt treats it as display text.
- `crates/application/src/lib.rs`, `crates/application/src/commands.rs` — export knowledge queries; add `Command::CreateNoteWithTitle`.
- `crates/application/src/ports.rs` — `Environment::local_day_bounds_millis` for the Today collection.
- `crates/application/src/library.rs` — own and maintain the index across open/create/save/state/import/restore.
- `crates/platform/src/environment.rs` (and `crates/platform/Cargo.toml` only if the compiler requires a `windows-sys` feature) — local-day bounds from the OS timezone.
- `apps/desktop/Cargo.toml` and `Cargo.lock` — add pinned `egui_extras =0.37.0` with `svg`; desktop version becomes `0.3.0` in Task 14 only.
- `apps/desktop/src/lib.rs`, `apps/desktop/src/app.rs`, `apps/desktop/src/worker.rs`, `apps/desktop/src/draft.rs`, `apps/desktop/src/view_state.rs` — query channel, controller composition, focus mode, session-only UI state.
- `apps/desktop/src/views/editor.rs`, `apps/desktop/src/views/preview.rs`, `apps/desktop/src/views/library.rs`, `apps/desktop/src/views/note_list.rs` — quiet icon chrome, wiki-link preview activation, collection/tag integration.
- `packaging/windows/crane.nsi`, `scripts/build-windows.ps1`, `README.md`, `docs/migration/windows-acceptance.md` — 0.3.0 installer name, documented shortcuts/features, and Windows evidence rows (Task 14 only).

`crates/domain/src/model.rs`, `crates/domain/src/query.rs`, `crates/domain/src/backup.rs`, `crates/storage/**`, and all `fixtures/compat/v1/**` are not modified. Legacy `query_note_ids` stays for compatibility; the 0.3 note list reads index-backed IDs from Task 11 onward.

---

### Task 1: Domain wiki-link and heading parser

**Files:**
- Create: `crates/domain/src/knowledge/mod.rs`, `crates/domain/src/knowledge/parser.rs`
- Modify: `crates/domain/src/lib.rs`
- Test: `crates/domain/tests/knowledge_parser.rs`

**Interfaces:**
- Consumes: `Note` from `crates/domain/src/model.rs` (read-only; no model change).
- Produces: `normalize_title(raw: &str) -> String`; `normalize_heading_anchor(raw: &str) -> String`; `HeadingFact { level: u8, text: String, normalized: String, anchor: String, range: Range<usize> }`; `HeadingTarget { raw: String, normalized: String, anchor_base: String }`; `WikiLink { range: Range<usize>, target_title: String, normalized_title: String, heading: Option<HeadingTarget>, alias: Option<String>, display_text: String }`; `parse_headings(body: &str) -> Vec<HeadingFact>`; `parse_wiki_links(body: &str) -> Vec<WikiLink>`. Later tasks use exactly these names.

Pinned grammar decisions (from the spec, made exact here): a backslash escapes the next character for delimiter purposes (`\|`, `\#`, `\]`, `\[[` are not delimiters; `\\` is an escaped backslash). The first unescaped `#` before the first unescaped `|` starts the heading; `[[Title#]]` means no heading. `display_text` is the trimmed non-empty alias, otherwise the trimmed target title. Empty titles, nested `[[`, and unmatched `]]` yield no link. Empty/whitespace-only titles are still returned by `parse_headings` callers' notes but Task 4 excludes them from the title index.

- [ ] **Step 1: Write the failing test**

Table-driven tests in `knowledge_parser.rs` covering: all four spec forms; alias/heading splitting; `\[[`, `\|`, `\#`, `\\[[Title]]` (this one *is* a link); inline code `` `[[X]]` `` and fenced/indented code exclusion; CRLF ranges; duplicate headings (`Intro`, `Intro`, literal `Intro-2` → anchors `intro`, `intro-2`, `intro-2-2`); punctuation kept in titles; Unicode lowercase (`[[ÀÉÎ]]` matches `àéî`); malformed inputs return empty vec and the source string is unchanged.

```rust
#[test]
fn parses_alias_and_heading_with_byte_range() {
    let links = parse_wiki_links("See [[Field Notes#On the desk|my notes]] end");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_title, "Field Notes");
    assert_eq!(links[0].heading.as_ref().unwrap().raw, "On the desk");
    assert_eq!(links[0].display_text, "my notes");
    assert_eq!(&"See [[Field Notes#On the desk|my notes]] end"[links[0].range.clone()], "[[Field Notes#On the desk|my notes]]");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-domain --test knowledge_parser --locked`
Expected: FAIL — `knowledge` module / `parse_wiki_links` not found (compile error).

- [ ] **Step 3: Implement the parser in `crates/domain/src/knowledge/parser.rs`**

Line scanner tracking fenced-code state (``` ``` ``` / `~~~`, closing fence at least as long) and inline-code spans (matching backtick runs); ATX headings (`#`–`######` + space, not 4-space indented) outside fences; wiki scan outside code only. Headings get occurrence suffixes after anchor normalization. No `unsafe`, no regex crate, no mutation of input.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-domain --test knowledge_parser --locked`
Expected: PASS, including a no-panic sweep over arbitrary UTF-8 fragments (`\u{10FFFF}`, combining marks, lone brackets).

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/lib.rs crates/domain/src/knowledge/mod.rs crates/domain/src/knowledge/parser.rs crates/domain/tests/knowledge_parser.rs
git commit -m "feat(domain): parse wiki links and headings for knowledge index"
```

### Task 2: Domain statistics and per-note facts

**Files:**
- Create: `crates/domain/src/knowledge/statistics.rs`
- Modify: `crates/domain/src/knowledge/mod.rs`
- Test: `crates/domain/tests/knowledge_statistics.rs`

**Interfaces:**
- Consumes: `parse_headings`, `parse_wiki_links` from Task 1; `extract_tags` from `model.rs`.
- Produces: `NoteStatistics { characters: usize, words: usize, paragraphs: usize, headings: usize, tasks: usize, completed_tasks: usize, reading_minutes: u64 }`; `note_statistics(body: &str) -> NoteStatistics`; `NoteFacts { note_id: String, title: String, normalized_title: String, headings: Vec<HeadingFact>, links: Vec<WikiLink>, tags: Vec<String>, statistics: NoteStatistics, search_body: String }`; `parse_note_facts(note: &Note) -> NoteFacts`; `parse_draft_facts(note_id: &str, title: &str, body: &str) -> NoteFacts` (tags derived with `extract_tags(body)`; used later for unsaved inspector data). `search_body` is the Unicode-lowercased body, stored once per note.

Counting rules (spec-exact): characters = Unicode scalar count; words = whitespace-delimited runs; paragraphs/headings/tasks from pulldown-cmark with `Options::ENABLE_TASKLISTS` (`Start(Paragraph)`, `Start(Heading)`, `TaskListMarker`); reading time = `ceil(words / 200)`, `0` words → `0`, any non-empty → at least `1`.

- [ ] **Step 1: Write the failing test**

Tests: empty body → all zeros; `# T\n\nOne two.\n\n- [ ] a\n- [x] b` → headings 1, tasks 2, completed 1, words counted exactly; 201 words → 2 minutes; tasks/links inside fenced code excluded from task counts and `links`; `parse_note_facts` keeps stored `note.tags`, `parse_draft_facts` re-derives them; source body string is byte-identical after parsing.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-domain --test knowledge_statistics --locked`
Expected: FAIL — `NoteFacts` / `note_statistics` not found.

- [ ] **Step 3: Implement `note_statistics` and the two fact constructors in `statistics.rs`/`mod.rs`**

Reuse the already-pinned `pulldown-cmark` for counts; delegate headings/links to Task 1 so byte ranges and statistics never diverge. Do not add dependencies.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-domain --test knowledge_statistics --test knowledge_parser --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/knowledge/mod.rs crates/domain/src/knowledge/statistics.rs crates/domain/tests/knowledge_statistics.rs
git commit -m "feat(domain): compute note statistics and per-note knowledge facts"
```

### Task 3: Domain hierarchical tag projection

**Files:**
- Create: `crates/domain/src/knowledge/tags.rs`
- Modify: `crates/domain/src/knowledge/mod.rs`
- Test: `crates/domain/tests/knowledge_tags.rs`

**Interfaces:**
- Consumes: stored `Note.tags` strings (already lowercased by `extract_tags`); no re-extraction here.
- Produces: `TagNode { path: String, name: String, depth: usize, note_count: usize, children: Vec<TagNode> }`; `build_tag_tree(tag_sets: &[Vec<String>]) -> Vec<TagNode>` (one inner vec per visible note, sorted by `name` at every level); `tag_in_subtree(tag: &str, prefix: &str) -> bool` (`tag == prefix || tag.starts_with(prefix + "/")`, case-insensitive on lowercased inputs).

- [ ] **Step 1: Write the failing test**

Tests: `project/crane/ui` creates three nested nodes; a note with `project/crane` + `project/web` counts once at `project`; sibling order is alphabetical; empty input → empty tree; selecting `project` via `tag_in_subtree` includes descendants but not `projectx`; duplicate tags on one note count once.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-domain --test knowledge_tags --locked`
Expected: FAIL — `build_tag_tree` not found.

- [ ] **Step 3: Implement `build_tag_tree` and `tag_in_subtree` in `tags.rs`**

Build with per-node `HashSet<usize>` of note indices for distinct counts, then materialize sorted `TagNode`s. Pure function; no persistence change.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-domain --test knowledge_tags --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/knowledge/mod.rs crates/domain/src/knowledge/tags.rs crates/domain/tests/knowledge_tags.rs
git commit -m "feat(domain): project slash tags into a counted hierarchy"
```

### Task 4: Domain cross-note knowledge index

**Files:**
- Create: `crates/domain/src/knowledge/index.rs`
- Modify: `crates/domain/src/knowledge/mod.rs`
- Test: `crates/domain/tests/knowledge_index.rs`

**Interfaces:**
- Consumes: `NoteFacts`, `Store`, `Note` from Tasks 1–2.
- Produces: `NoteVisibility { pinned: bool, archived: bool, trashed: bool, updated_at: u64 }`; `HeadingResolution::Found { anchor: String, range: Range<usize> } | Missing | NotRequested`; `Resolution::Resolved { note_id: String, heading: HeadingResolution } | Unresolved { normalized_title: String } | Ambiguous { candidate_ids: Vec<String> }`; `InboundLink { source_note_id: String, source_title: String, range: Range<usize>, heading: Option<HeadingTarget>, display_text: String, context_excerpt: String }`; `KnowledgeIndex::build(store: &Store) -> Self`; `replace_note(&mut self, note: &Note)`; `update_visibility(&mut self, note: &Note)` (no reparse when title/body/tags are unchanged); `facts(&self, note_id: &str) -> Option<&NoteFacts>`; `visibility(&self, note_id: &str) -> Option<&NoteVisibility>`; `resolve(&self, link: &WikiLink) -> Resolution`; `backlinks(&self, note_id: &str) -> Vec<InboundLink>`; `context_excerpt(body: &str, range: &Range<usize>) -> String` (40 chars each side, Unicode-boundary safe, whitespace collapsed). Notes with empty normalized titles are stored as facts but never enter the title map. `backlinks` includes only uniquely resolved inbound links.

- [ ] **Step 1: Write the failing test**

Tests: unique match resolves with heading `Found`/`Missing`; duplicate titles (case/whitespace variants) → `Ambiguous` with IDs in store order; unknown title → `Unresolved`; renamed note (replace with new title) makes old inbound links unresolved and drops stale backlinks; `replace_note` on one note leaves other facts untouched; `update_visibility` flips trash without changing `links`; backlink excerpt is bounded and inert; rebuild from the same store equals incremental state; trashed/archived notes keep facts.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-domain --test knowledge_index --locked`
Expected: FAIL — `KnowledgeIndex` not found.

- [ ] **Step 3: Implement `KnowledgeIndex` in `index.rs`**

Maps: `facts: HashMap<String, NoteFacts>`, `visibility: HashMap<String, NoteVisibility>`, `by_title: HashMap<String, Vec<String>>`, `inbound: HashMap<String, Vec<InboundLink>>` keyed by normalized title. `replace_note` removes the note's old outbound contributions before inserting new ones. Deterministic iteration: sort candidate/backlink outputs by `(source updated_at desc, source_note_id)` at query time in Task 6, not by hash order here.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-domain --locked`
Expected: PASS for the whole domain crate, including untouched `compatibility`, `backups`, `markdown`, `preview`, `queries` suites.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/knowledge/mod.rs crates/domain/src/knowledge/index.rs crates/domain/tests/knowledge_index.rs
git commit -m "feat(domain): add cross-note knowledge index with backlinks"
```

### Task 5: Application index ownership, derived collections, and create-with-title

**Files:**
- Create: `crates/application/src/knowledge.rs` (types only in this task)
- Modify: `crates/application/src/lib.rs`, `crates/application/src/commands.rs`, `crates/application/src/ports.rs`, `crates/application/src/library.rs`, `crates/platform/src/environment.rs`
- Test: `crates/application/tests/knowledge.rs` (lifecycle tests; query-ranking tests land in Task 6)

**Interfaces:**
- Consumes: `KnowledgeIndex`, `NoteFacts` from Tasks 2/4.
- Produces: `CollectionView::Notes | Pinned | Today | Todos | Untagged | Archive | Trash` (`#[derive(Clone, Copy, Debug, PartialEq, Eq)]`); `Command::CreateNoteWithTitle { title: String }` returning existing `CommandData::Created(Note)`; `Environment::local_day_bounds_millis(&self) -> (u64, u64)` (start inclusive, end exclusive, OS local time); `Library::knowledge_index(&self) -> &KnowledgeIndex` (read-only, for queries/tests); index maintenance inside `Library::execute` — full build in `Library::open`, `replace_note` after successful `Created`/`Saved`, `update_visibility` after successful `StateChanged`, full rebuild after successful `Imported`/`Restored`. A failed `repository.commit` leaves both store and index untouched because index updates run only after `commit_candidate`/`restore` persistence succeeds and before the `Ok` result is returned.

- [ ] **Step 1: Write the failing test**

In `crates/application/tests/knowledge.rs` reusing `tests/support`: open the extended v1 fixture → index facts count equals note count; execute `CreateNoteWithTitle { title: "  Linked Note  " }` → created title is `Linked Note` and index resolves it; `SaveNote` retitling a note updates resolution; `SetNoteState` trash/restore flips `visibility` without dropping facts; injected commit failure (existing `fail_next_commit`) leaves `snapshot()` and index resolution unchanged; empty/whitespace title → `ErrorCode::Validation` with no commit. Update `FakeEnvironment` in `crates/application/tests/support/mod.rs` and the desktop test environment to return a fixed day bound (e.g. `(0, u64::MAX)` until Task 6 sets real bounds).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-application --test knowledge --locked`
Expected: FAIL — `CollectionView` / `CreateNoteWithTitle` / `knowledge_index` not found.

- [ ] **Step 3: Implement ownership in the application layer**

Add the `Environment` method (platform implementation uses the OS local timezone; add a `windows-sys` feature to `crates/platform/Cargo.toml` only if the compiler demands it), trim-and-validate the new command through existing `validate_note`, and wire index updates at the exact commit boundaries above. `RestoreBackup` updates the index only after safety-backup + replacement commit succeed. No storage, egui, or filesystem code enters `crane-application` beyond the existing ports.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-application --locked`
Expected: PASS, including existing `transactions` and `transfers` suites (failed-save preservation unchanged).

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/lib.rs crates/application/src/commands.rs crates/application/src/ports.rs crates/application/src/library.rs crates/application/src/knowledge.rs crates/application/tests/knowledge.rs crates/application/tests/support/mod.rs crates/platform/src/environment.rs apps/desktop/tests/common/mod.rs
git commit -m "feat(application): own memory-only knowledge index in Library"
```

### Task 6: Application knowledge queries (Quick Open, resolution, completion)

**Files:**
- Modify: `crates/application/src/knowledge.rs`, `crates/application/src/library.rs`, `crates/application/src/lib.rs`
- Test: `crates/application/tests/knowledge.rs`

**Interfaces:**
- Consumes: `KnowledgeIndex` (Task 4), `CollectionView` (Task 5), `excerpt_from` from `crates/domain/src/markdown.rs`.
- Produces: `Query::QuickOpen { query: String, include_archived: bool } | CompleteWikiLink { fragment: String } | ResolveWikiLink { link: WikiLink } | KnowledgeFor { note_id: String } | TagTree { include_archived: bool } | NotesForCollection { collection: CollectionView, tag_path: Option<String>, query: String }`; `QueryData::QuickOpen(Vec<QuickOpenHit>) | Completion(Vec<CompletionCandidate>) | Resolution(ResolutionView) | Knowledge(Box<KnowledgeView>) | TagTree(Vec<TagNode>) | NoteIds(Vec<String>)`; `QueryResponse { store_revision: u64, data: QueryData }`; `Library::query(&mut self, query: Query) -> QueryResponse` (`&mut` only for the consistency-rebuild fallback in Step 3). Read models: `QuickOpenHit { note_id: String, title: String, excerpt: String, updated_at: u64, pinned: bool, archived: bool }`; `CompletionCandidate::Title { note_id: String, title: String } | Heading { note_id: String, heading_text: String, anchor: String }` with `insert_text()` (`Title` for titles, `#Heading` suffix form for headings as pinned by tests); `ResolutionView::Resolved { note_id: String, heading: HeadingResolution } | Unresolved { target_title: String } | Ambiguous { candidates: Vec<AmbiguousCandidate> }`, `AmbiguousCandidate { note_id: String, title: String, excerpt: String, updated_at: u64 }`; `KnowledgeView { statistics: NoteStatistics, outline: Vec<OutlineItem>, outgoing: Vec<OutgoingLinkView>, backlinks: Vec<BacklinkView> }`, `OutlineItem { level: u8, text: String, anchor: String, range: Range<usize> }`, `OutgoingLinkView { link: WikiLink, state: LinkState }`, `LinkState::Resolved { note_id: String, heading_found: bool } | Unresolved | Ambiguous`, `BacklinkView { source_note_id: String, source_title: String, display_text: String, context_excerpt: String, range: Range<usize> }`. Constants: `QUICK_OPEN_LIMIT: usize = 50`, `COMPLETION_LIMIT: usize = 8`. Completion and Quick Open exclude trashed notes; Quick Open excludes archived unless `include_archived`. Candidates/backlinks are re-sorted by store order (from `Library.store`) where the spec requires non-arbitrary order.

Ranking (spec-exact): blank Quick Open → pinned first, then `updated_at` desc, then note ID asc. Non-blank → rank class exact title, title prefix, title substring, tag substring, body substring (in that order), then pinned, `updated_at` desc, note ID asc. `NotesForCollection`: collection predicate (`Today` via `local_day_bounds_millis` on `updated_at`; `Todos` via `statistics.tasks > completed_tasks` equivalent unchecked count; `Untagged` via empty facts tags), then `tag_path` subtree via `tag_in_subtree`, then case-insensitive title/body/tag substring, ordered pinned desc, `updated_at` desc, store order.

- [ ] **Step 1: Write the failing test**

Tests: deterministic ranking across all five classes with tie-breaks; blank-query ordering and 50-hit cap; archived toggle and trash exclusion; ambiguous resolution returns excerpts in store order and never a single pick; missing heading → `Resolved` with `HeadingResolution::Missing`; `KnowledgeFor` on a note with unresolved/ambiguous outgoing links states each explicitly and claims no ambiguous backlink; `NotesForCollection` for Today (fake day bounds), Todos, Untagged, parent-tag subtree, and text query; `TagTree` counts match Task 3 on the same corpus; stale safety — `query` after a failed save answers from the last committed revision.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-application --test knowledge --locked`
Expected: FAIL — `Query` / `Library::query` not found.

- [ ] **Step 3: Implement `Library::query` in `crates/application/src/knowledge.rs`**

Pure reads over `self.store` + `self.index`; no I/O, excerpts via `excerpt_from(body, 120)` and domain `context_excerpt`. Consistency fallback (spec failure rule): `Library::query` takes `&mut self` if the facts/visibility counts ever differ from the store, it fully rebuilds the index before answering — implement as `fn ensure_index_consistent(&mut self)` called at the top of `query`, keeping the common path allocation-free. `CompletionCandidate::insert_text()` returns the title for `Title` and the heading text for `Heading`; the desktop rebuilds the full `[[…]]` from the fragment (Task 11). Heading completion: when the fragment's title part uniquely resolves, return that note's headings filtered by the heading fragment; otherwise return title candidates. `|` in the fragment yields no candidates (alias editing).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-application --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/lib.rs crates/application/src/knowledge.rs crates/application/src/library.rs crates/application/tests/knowledge.rs
git commit -m "feat(application): add typed knowledge queries with deterministic ranking"
```

### Task 7: Preview wiki-link inline

**Files:**
- Modify: `crates/domain/src/preview.rs`, `crates/domain/src/markdown.rs`, `crates/domain/tests/preview.rs` (exhaustive-match helper only)
- Test: `crates/domain/tests/preview.rs`

**Interfaces:**
- Consumes: `parse_wiki_links` from Task 1 (same body string, so ranges align).
- Produces: `Inline::WikiLink(WikiLink)` in `crates/domain/src/preview.rs`. `parse_preview` splits `Event::Text` at wiki-link byte ranges outside code (links inside `Event::Code`/code blocks never appear because the Task 1 scanner already excludes them; inline-code text events are `Code`, not `Text`). `excerpt_from` renders a wiki link as its `display_text`.

- [ ] **Step 1: Write the failing test**

Tests: paragraph `See [[Field Notes|notes]]` yields `Text`, `WikiLink`, `Text` in order with alias display; `[[A#H]]` in a heading is a wiki link; `` `[[X]]` `` and fenced `[[X]]` stay literal text/code; excerpt contains the alias, not brackets; existing safe-link/unsafe-scheme tests unchanged.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-domain --test preview --locked`
Expected: FAIL — `Inline::WikiLink` not found / wiki text returned as plain `Text`.

- [ ] **Step 3: Implement range-splitting in `parse_preview`**

Compute `parse_wiki_links(markdown)` once, track the running byte offset of text events, and emit `WikiLink` segments. Update every exhaustive `Inline` match this crate owns (`preview.rs`, `markdown.rs` excerpt) — desktop rendering follows in Task 11.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-domain --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/preview.rs crates/domain/src/markdown.rs crates/domain/tests/preview.rs
git commit -m "feat(domain): surface wiki links as typed preview inlines"
```

### Task 8: Lucide vendoring, renderer spike, and icon catalog

**Files:**
- Create: `apps/desktop/assets/icons/lucide/*.svg`, `apps/desktop/assets/icons/lucide/manifest.json`, `apps/desktop/src/icons.rs`, `THIRD_PARTY_NOTICES.md`
- Modify: `apps/desktop/Cargo.toml`, `Cargo.lock`, `apps/desktop/src/lib.rs`, `apps/desktop/src/main.rs` (one-time loader install)
- Test: `apps/desktop/tests/icons.rs`

**Interfaces:**
- Consumes: pinned egui/eframe 0.37.0; `egui_extras::install_image_loaders` called once at startup.
- Produces: `IconId` enum with exactly these variants, each used by the named shipped control (add a variant only with its shipped control in the same later task): `Plus` (new note), `Search` (Quick Open), `Info` (inspector), `X` (close), `ChevronDown/ChevronRight` (tag fold), `ArrowLeft` (back), `Pin/Archive/Trash2/Eye/Pencil` (editor chrome), `Bold, Italic, Code, List, ListOrdered, ListChecks, Quote, Link, Heading` (formatting; `ListChecks` also Todos), `AlertTriangle` (unresolved-link warning, always with text), `PanelLeft` (focus mode), `FileText` (Notes), `Calendar` (Today), `Tag` (Untagged/tags); `icon_bytes(icon: IconId) -> &'static [u8]` via `include_bytes!`; `icon_button(ui: &mut egui::Ui, icon: IconId, accessible_name: &str, tooltip: &str) -> egui::Response` (real `Button` with image, 38pt min size, `accessible_name` set). Manifest schema: a JSON object with `lucideVersion` (the vendored tag string), `source` (the Lucide repository URL), and `icons` (objects with `file`, `lucideName`, `sha256` as 64 lowercase hex characters, and `featherDerived` boolean) — one entry per vendored file, e.g. `plus.svg` / `plus`. The vendored tag is the newest Lucide release whose SVGs the spike renders correctly; record the tag actually vendored — do not assert a version that was not downloaded and hashed in this task.

- [ ] **Step 1: Write the failing test**

Tests in `icons.rs`: manifest parses, every `IconId` file exists in the manifest with a 64-hex SHA-256 matching the embedded bytes, every manifest icon is referenced by an `IconId` (no unused assets), each SVG starts with `<svg` and contains no `http://`/`https://` reference, `THIRD_PARTY_NOTICES.md` contains the ISC heading and names every `featherDerived: true` icon; a `Harness` renders one `icon_button` and finds it by AccessKit label.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test icons --locked`
Expected: FAIL — assets / `icons` module not found.

- [ ] **Step 3: Spike, vendor, and implement**

First add `egui_extras = { version = "=0.37.0", default-features = false, features = ["svg"] }`, call `install_image_loaders`, and render one vendored SVG in the test harness. Gate: if the SVG does not rasterize offline at 16/18pt with Crane colors, stop and report — do not silently switch renderers. Then download the chosen Lucide release archive once (development-time only), copy only the listed icons, hash them into `manifest.json`, write full ISC + Feather MIT notices (verbatim license texts, Feather icon names that intersect the shipped set), and implement `icons.rs` with Crane-colored 16–18pt artwork and ~1.75 stroke via the SVG loader URI. Record the binary-size delta (`cargo build --release` before/after) in the commit body.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test icons --locked` and `cargo build -p crane-desktop --locked --offline`
Expected: PASS; offline build succeeds with no fetch.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/Cargo.toml Cargo.lock apps/desktop/src/lib.rs apps/desktop/src/main.rs apps/desktop/src/icons.rs apps/desktop/assets/icons/lucide THIRD_PARTY_NOTICES.md apps/desktop/tests/icons.rs
git commit -m "feat(desktop): vendor pinned Lucide subset with offline SVG rendering"
```

### Task 9: Desktop query channel, module extraction, and sidebar

**Files:**
- Create: `apps/desktop/src/sidebar.rs`
- Modify: `apps/desktop/src/worker.rs`, `apps/desktop/src/draft.rs`, `apps/desktop/src/app.rs`, `apps/desktop/src/lib.rs`, `apps/desktop/src/view_state.rs`, `apps/desktop/src/views/library.rs`, `apps/desktop/src/views/note_list.rs`, `crates/application/src/knowledge.rs` (add counts query)
- Test: `apps/desktop/tests/navigation.rs`

**Interfaces:**
- Consumes: `Query`/`QueryData` (Task 6), `TagNode` (Task 3), `IconId::Plus/Search/PanelLeft` (Task 8).
- Produces: `WorkerRequestKind::Command(Command) | Query(Query)` and `WorkerOutcome::Command(Result<CommandResult, AppError>) | Query(QueryResponse)` in `worker.rs` (`WorkerRequest { request_id: u64, kind: WorkerRequestKind }`, `WorkerResult { request_id: u64, outcome: WorkerOutcome }`; `DraftCoordinator` only ever sends/inspects `Command` outcomes). `CraneApp::send_query(query: Query) -> u64` returns the request ID; query responses with an unknown/superseded ID are dropped. `SidebarState { expanded_tags: HashSet<String> }` (session-only); `SidebarIntent::Create | Collection(CollectionView) | Tag(Option<String>) | QuickOpen`; `sidebar::show(ui, counts: &SidebarCounts, tag_tree: &[TagNode], state: &mut SidebarState, active: CollectionView, active_tag: Option<&str>, collapsed: bool) -> Option<SidebarIntent>`. New application query: `Query::CollectionCounts -> QueryData::CollectionCounts(SidebarCounts)` with `SidebarCounts { notes: usize, pinned: usize, today: usize, todos: usize, untagged: usize, archive: usize, trash: usize }`. `ViewState.collection` becomes `CollectionView`; the note list IDs come from `Query::NotesForCollection` responses cached by `(store_revision, collection, tag_path, query)` in `CraneApp` — `has_pending_work()` includes in-flight queries so test `settle()` stays correct.

This task is extraction + wiring only: existing Notes/Pinned/Archive/Trash behavior, search, pin/archive/trash flows, and layout breakpoints must pass unchanged before Today/Todos/Untagged rows are enabled by the counts response.

- [ ] **Step 1: Write the failing test**

Extend `navigation.rs`: all pre-existing navigation/search/state tests pass against the new channel (they fail first on the `WorkerRequest` shape); sidebar shows `Today`, `Todos`, `Untagged` with counts from a fixture containing one of each; expanding `project` reveals `project/crane` and selecting the parent lists descendant-tagged notes; fold state survives a frame but is not persisted (new `CraneApp` starts collapsed).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test navigation --locked`
Expected: FAIL — `WorkerRequestKind` / `sidebar` / `CollectionView` wiring not found.

- [ ] **Step 3: Implement the channel and sidebar**

`spawn_worker` matches on `WorkerRequestKind` (`library.execute` vs `library.query`) on the same serialized thread. Move rail rendering from `views/library.rs` into `sidebar.rs` (Data menu, theme, reduced-motion controls keep their existing accessible names and behavior), render tags recursively from `TagTree` with chevron icon buttons named `Expand <tag>` / `Collapse <tag>`, and switch the list to `NotesForCollection` IDs with the note-list heading showing the active collection name and result count. Cache `TagTree` and `CollectionCounts` responses in `CraneApp` by `store_revision` so sidebar rendering never re-queries per frame. Keep `app.rs` as composition owner; controllers emit intents only.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test navigation --test worker --test drafts --locked`
Expected: PASS with no behavior change to existing flows.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/worker.rs apps/desktop/src/draft.rs apps/desktop/src/app.rs apps/desktop/src/lib.rs apps/desktop/src/view_state.rs apps/desktop/src/sidebar.rs apps/desktop/src/views/library.rs apps/desktop/src/views/note_list.rs crates/application/src/knowledge.rs apps/desktop/tests/navigation.rs
git commit -m "feat(desktop): route knowledge queries over worker and extract sidebar"
```

### Task 10: Quick Open overlay

**Files:**
- Create: `apps/desktop/src/quick_open.rs`
- Modify: `apps/desktop/src/app.rs`, `apps/desktop/src/views/note_list.rs` (visible Quick Open control), `apps/desktop/src/lib.rs`
- Test: `apps/desktop/tests/quick_open.rs`

**Interfaces:**
- Consumes: `Query::QuickOpen` (Task 6), `IconId::Search` (Task 8), existing `DeferredAction::Select` draft-save gate in `app.rs`.
- Produces: `QuickOpenState { open: bool, query: String, include_archived: bool, hits: Vec<QuickOpenHit>, active: usize, sequence: u64, return_caret: Option<(String, Range<usize>)> }`; `QuickOpenIntent::OpenNote(String) | None`; `quick_open::show(ctx: &egui::Context, state: &mut QuickOpenState, on_query: &mut dyn FnMut(Query)) -> Option<QuickOpenIntent>`. Visible control: an icon button named `Quick Open` in the note-list header next to search, plus `Ctrl+O` in `CraneApp::handle_shortcuts`. Archived toggle is a checkbox named `Include archived` inside the overlay. Opening a hit goes through `request_after_save(DeferredAction::Select)`; the overlay closes on Enter, and if the required save fails the draft stays selected with the existing inline Retry (pinned by test).

- [ ] **Step 1: Write the failing test**

Kittest cases: `Ctrl+O` opens and focuses `Quick Open` search; typing filters via worker hits; ArrowDown/Enter opens the second hit and focus returns to the note body; Escape closes with no selection change; blank query shows pinned-first recents capped at 50; `Include archived` reveals an archived hit while trash never appears; editing the draft then opening a hit with an injected save failure keeps the current note and shows the save error; a response for sequence *n* arriving after the query changed to *n+1* does not overwrite hits.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test quick_open --locked`
Expected: FAIL — `quick_open` module / `Quick Open` control not found.

- [ ] **Step 3: Implement the overlay**

Centered `egui::Window`/area with a single text field, bounded hit list (virtualized by `ScrollArea` + clipped rows), keyboard handling before the editor sees the keys, and focus restoration to the stored editor caret. Queries are sent only when `query`/`include_archived` changes; stale responses are dropped by sequence. No fuzzy matching beyond Task 6 ranking.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test quick_open --test navigation --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/quick_open.rs apps/desktop/src/app.rs apps/desktop/src/lib.rs apps/desktop/src/views/note_list.rs apps/desktop/tests/quick_open.rs
git commit -m "feat(desktop): add keyboard-first Quick Open overlay"
```

### Task 11: Wiki-link completion, preview navigation, and chooser

**Files:**
- Create: `apps/desktop/src/knowledge_navigation.rs`
- Modify: `apps/desktop/src/app.rs`, `apps/desktop/src/views/editor.rs`, `apps/desktop/src/views/preview.rs`, `apps/desktop/src/lib.rs`
- Test: `apps/desktop/tests/knowledge_navigation.rs`

**Interfaces:**
- Consumes: `Query::CompleteWikiLink`/`ResolveWikiLink` (Task 6), `Inline::WikiLink` (Task 7), `Command::CreateNoteWithTitle` (Task 5).
- Produces: `wiki_fragment_at(body: &str, caret_byte: usize) -> Option<WikiFragment>` with `WikiFragment { replace_range: Range<usize>, raw: String }` (caret after an unescaped `[[` with no closing `]]`, outside code, not during IME composition — composition state read from egui `TextEditState`); `CompletionState { fragment: Option<WikiFragment>, candidates: Vec<CompletionCandidate>, active: usize, sequence: u64 }`; preview output becomes `PreviewAction::None | HttpError(String) | WikiLink(WikiLink)` from `preview::show`/`show_with_opener` (HTTP links keep using `crane_platform::open_http_link`); `KnowledgeNavIntent::OpenNote { note_id: String, heading: Option<HeadingResolution> } | CreateNote { title: String }`. Ambiguity/unresolved chooser is an egui window listing `AmbiguousCandidate`s (title, excerpt, modified time) or a single `Create note "<title>"` action; choosing create dispatches `CreateNoteWithTitle` through the draft-save gate, then applies the pending heading jump or `Heading not found` status. Heading jump in edit mode sets the body `TextEditState` char range to the heading range start and focuses the body; in preview it scrolls the matching heading row into view.

- [ ] **Step 1: Write the failing test**

Kittest cases: typing `[[Fie` shows title candidates and Enter produces `[[Field Notes]]` with caret after `]]`; `[[Field Notes#On` switches to heading candidates; `|` yields no popup; Escape leaves text unchanged; completion inside a fenced block never appears; clicking a wiki link in preview opens the note, an ambiguous link opens the chooser and only the clicked candidate navigates, an unresolved link offers create and the created note opens with its draft-save gate intact; resolved note with missing heading opens at top and status reads `Heading not found`; undo (`Ctrl+Z`) after completion insertion restores the pre-completion body.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test knowledge_navigation --locked`
Expected: FAIL — `knowledge_navigation` / `PreviewAction::WikiLink` handling not found.

- [ ] **Step 3: Implement completion and navigation**

Editor reads caret via `TextEditState::load(...).cursor.char_range()` (char→byte through `char_indices`), sends `CompleteWikiLink` on fragment change only, renders candidates in a popup area anchored below the text edit, and applies insertion by replacing `replace_range` and restoring a char range — mutating the same draft `String` the `TextEdit` owns so egui undo keeps working. Preview renders `WikiLink` as a link-styled button with accessible name `Open linked note: <display>`. Modifier-click in the source editor is not added (spec exclusion).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test knowledge_navigation --test editor --locked`
Expected: PASS, including existing Unicode save/switch editor tests.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/knowledge_navigation.rs apps/desktop/src/app.rs apps/desktop/src/lib.rs apps/desktop/src/views/editor.rs apps/desktop/src/views/preview.rs apps/desktop/tests/knowledge_navigation.rs
git commit -m "feat(desktop): add wiki-link completion and safe link navigation"
```

### Task 12: Information inspector

**Files:**
- Create: `apps/desktop/src/inspector.rs`
- Modify: `apps/desktop/src/app.rs`, `apps/desktop/src/views/editor.rs`, `apps/desktop/src/lib.rs`
- Test: `apps/desktop/tests/inspector.rs`

**Interfaces:**
- Consumes: `Query::KnowledgeFor` committed view (Task 6), `parse_draft_facts` (Task 2), `KnowledgeNavIntent` (Task 11), `IconId::Info/X` (Task 8).
- Produces: `InspectorTab::Statistics | Outline | Links | Backlinks`; `InspectorState { open: bool, tab: InspectorTab }` (session-only, default `Statistics`); `InspectorIntent::OpenNote { note_id: String, heading: Option<HeadingResolution> } | JumpToHeading { range: Range<usize> } | None`; `inspector::show(ui, state: &mut InspectorState, draft_facts: &NoteFacts, committed: Option<&KnowledgeView>, theme: ThemeChoice) -> InspectorIntent`. Draft facts are cached in `CraneApp` by `(note_id, draft.revision)` via `parse_draft_facts`; committed data comes only from `KnowledgeFor` on the committed revision. Statistics and Outline render `draft_facts`; Links and Backlinks render `committed` and show `Save to update links` when the draft is dirty. Dock rule: editor width `>= 760.0` allocates a 320pt right column inside the editor region; below that, an anchored floating window that `CraneApp` closes before any pane navigation. Empty states are literal: `No headings yet`, `No outgoing links`, `No backlinks yet`, `Link target not found`, `Multiple notes share this title`.

- [ ] **Step 1: Write the failing test**

Kittest cases: `Info` button (accessible name `Note information`) opens Statistics with exact draft word/task counts before save; Outline lists headings indented by level and clicking one moves the edit caret to the heading start; Links shows resolved/unresolved/ambiguous states distinctly; Backlinks shows source title + excerpt and clicking opens the source note through the draft-save gate; unsaved body text never appears as a backlink; tab choice survives switching notes within the session; at narrow editor width the inspector floats and closes when navigating to the list; all four empty states render.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test inspector --locked`
Expected: FAIL — `inspector` module / `Note information` control not found.

- [ ] **Step 3: Implement the inspector**

Four-tab column with real buttons for tabs (AccessKit names `Statistics`, `Outline`, `Links`, `Backlinks`), scrollable lists, and intents routed through Task 11 navigation. In the same `views/editor.rs` change, replace the visually heavy text action row with quiet icon controls (`Pin`, `Archive`, `Trash2`, `Eye`/`Pencil`, `Info`) that keep the existing accessible names verbatim (`Pin note`/`Unpin note`, `Archive note`, `Move to Trash`, `Preview Markdown`/`Edit Markdown`, `Note information`); `Save now`, `Export note`, and Retry stay text. No parsing in the desktop beyond the cached `parse_draft_facts` call; no body text is logged.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test inspector --test knowledge_navigation --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/inspector.rs apps/desktop/src/app.rs apps/desktop/src/lib.rs apps/desktop/src/views/editor.rs apps/desktop/tests/inspector.rs
git commit -m "feat(desktop): add Statistics Outline Links Backlinks inspector"
```

### Task 13: Formatting bar

**Files:**
- Create: `apps/desktop/src/formatting.rs`
- Modify: `apps/desktop/src/views/editor.rs`, `apps/desktop/src/app.rs`, `apps/desktop/src/lib.rs`
- Test: `apps/desktop/tests/formatting.rs`

**Interfaces:**
- Consumes: `IconId` formatting variants (Task 8), body `TextEditState` char range (Task 11 pattern). Only syntax the current preview renders is offered; no tables/highlight/media.
- Produces: `FormatAction::Heading(u8) | Task | BulletList | NumberedList | Quote | Bold | Italic | InlineCode | CodeBlock | InsertLink { url: String }`; `FormatOutcome { body: String, selection: Range<usize>, applied: bool }`; `apply_format(body: &str, selection: Range<usize>, action: &FormatAction) -> FormatOutcome` (pure, UTF-8-boundary safe, clamps selection); `FormattingState { link_open: bool, link_url: String }`; `formatting::show(ui, state: &mut FormattingState, enabled: bool) -> Option<FormatAction>`. Emitted Markdown, pinned by tests: Heading `## ` prefix (same level again removes it); Task `- [ ] `; Bullet `- `; Numbered `1. ` (incrementing per line); Quote `> `; Bold `**x**`; Italic `*x*`; InlineCode `` `x` ``; CodeBlock fenced with ``` on their own lines; InsertLink `[selection](url)` only for `http://`/`https://` URLs, otherwise `applied: false` and text unchanged. Line actions apply to every line intersecting the selection; inline actions wrap the selection or insert paired markers with the caret between them. Repeating Bold/Italic/InlineCode/line-prefix on an exactly wrapped/prefixed selection unwraps/removes it.

- [ ] **Step 1: Write the failing test**

Pure tests for every action above: wrap, empty-selection caret placement, multi-line prefixes, toggle-off, link rejection for `file:`/`javascript:`/relative URLs, selection clamped inside a multi-byte character without panic. Kittest: select body text, click `Bold`, draft body and committed-after-save body both show `**…**`; `Ctrl+Z` restores the prior body; formatting with a dirty draft then switching notes still passes through the draft-save gate.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test formatting --locked`
Expected: FAIL — `formatting` module / formatting controls not found.

- [ ] **Step 3: Implement transformations and the bar**

`apply_format` works on byte ranges converted from egui char ranges at the call site. The bar sits at the lower edge of the editor (hidden in preview), icon buttons carry accessible names (`Heading`, `Task list`, `Bulleted list`, `Numbered list`, `Quote`, `Bold`, `Italic`, `Inline code`, `Code block`, `Insert link`), destructive/state meaning is never color-only, and edits mutate the draft `String` in place then restore the outcome char range so undo, IME, and clipboard state survive.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --test formatting --test editor --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/formatting.rs apps/desktop/src/lib.rs apps/desktop/src/views/editor.rs apps/desktop/src/app.rs apps/desktop/tests/formatting.rs
git commit -m "feat(desktop): add selection-aware Markdown formatting bar"
```

### Task 14: Focus mode, responsive refinement, and accessibility hardening

**Files:**
- Modify: `apps/desktop/src/app.rs`, `apps/desktop/src/view_state.rs`, `apps/desktop/src/sidebar.rs`, `apps/desktop/src/inspector.rs`
- Test: `apps/desktop/tests/navigation.rs`, `apps/desktop/tests/accessibility.rs`

**Interfaces:**
- Consumes: Tasks 9–13 controllers and their intents.
- Produces: `ViewState.focus_mode: bool` (session-only). Toggle is the `PanelLeft` icon button named `Toggle focus mode` in the editor chrome; entering hides rail + list, leaving restores prior `LayoutMode` widths and selection (widths are the existing `theme` constants — nothing is persisted). Escape priority, implemented once in `CraneApp`: completion → Quick Open → chooser → floating inspector → exit focus mode → single-pane back. Focus order is rail, list, editor chrome, title, body, formatting bar, inspector; overlays move focus in and restore the initiating caret/control on close. No decorative animation is added; reduced-motion behavior from settings is untouched.

- [ ] **Step 1: Write the failing test**

Kittest cases: focus mode hides `Notes` rail/list labels while the editor remains, toggling off restores them with the same selected note; Escape exits focus mode only after any open overlay/inspector has closed; wide (>900), collapsed-rail (681–900), and one-pane (≤680) layouts in both themes keep note rows rectangular, targets ≥44pt in compact mode, and every icon control findable by AccessKit role+name (`New note`, `Quick Open`, `Note information`, `Toggle focus mode`, formatting names, `Expand <tag>`); Tab traversal reaches search, title, body, formatting bar, and inspector in that order; screen-reader names never come from icon artwork alone (each icon button also has a tooltip with the same text).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-desktop --test navigation --test accessibility --locked`
Expected: FAIL — `Toggle focus mode` / focus-mode behavior not found.

- [ ] **Step 3: Implement focus mode and the Escape stack**

Branch layout in `CraneApp::show` on `focus_mode`, centralize transient-surface dismissal in one function used by both shortcut handling and pane navigation, and audit every icon control added in Tasks 8–13 for name/tooltip/focus visibility. Screenshot Rect assertions (pane widths 236/330 wide, 72/290 collapsed, single pane ≤680) use harness node rects; screenshots, if captured, supplement but never replace these assertions.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crane-desktop --locked`
Expected: PASS for all desktop suites (`accessibility`, `drafts`, `editor`, `navigation`, `quick_open`, `knowledge_navigation`, `inspector`, `formatting`, `icons`, `restore_dialog`, `startup`, `transfers`, `worker`).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/app.rs apps/desktop/src/view_state.rs apps/desktop/src/sidebar.rs apps/desktop/src/inspector.rs apps/desktop/tests/navigation.rs apps/desktop/tests/accessibility.rs
git commit -m "feat(desktop): add focus mode and harden responsive accessibility"
```

### Task 15: Compatibility, large-library evidence, and Windows release validation

**Files:**
- Create: `crates/application/tests/knowledge_perf.rs`
- Modify: `apps/desktop/Cargo.toml` (version `0.3.0`), `packaging/windows/crane.nsi`, `scripts/build-windows.ps1` (installer artifact name `Crane_0.3.0_x64-setup.exe`), `README.md`, `docs/migration/windows-acceptance.md`
- Test: all suites below; no fixture under `fixtures/compat/v1/` is modified.

**Interfaces:**
- Consumes: every prior task. Produces release evidence only — no new runtime behavior. Backup `app_version` now reports `0.3.0` while `BACKUP_VERSION` and `SCHEMA_VERSION` stay `1` (inspection ignores `app_version`; pinned by test).

- [ ] **Step 1: Write the failing test**

`knowledge_perf.rs` builds a deterministic 5,000-note store (mixed `[[links]]`, nested tags, tasks; one body near 64 KiB) and asserts: full index build indexes 5,000 facts, 200 Quick Open queries return bounded hits, one `SaveNote` replaces exactly one note's facts (asserted via facts/outgoing counts, not wall-clock), and timings are emitted with `eprintln!` as diagnostics. Compatibility assertions: each v1 fixture opens through `Library`, `serde_json::to_value(snapshot.store)` equals the fixture value, a backup created and restored from the fixture store round-trips with identical store JSON. Separately, before committing this task, `git status --porcelain fixtures/` must print nothing.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crane-application --test knowledge_perf --locked`
Expected: FAIL — perf suite not found (before it exists); after writing, any count assertion failure is a real defect to debug, not a timing flake.

- [ ] **Step 3: Run the complete repository gate**

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --all-targets --all-features --locked
cargo build -p crane-desktop --locked --offline
powershell -NoProfile -File scripts/check-layers.ps1
powershell -NoProfile -File scripts/check-layers.ps1 -SelfTest
powershell -NoProfile -File scripts/check-native-release.ps1
powershell -NoProfile -File scripts/check-native-release.ps1 -SelfTest
powershell -NoProfile -File scripts/smoke-windows.ps1 -SelfTest
```

Expected: every command exits 0; layer and native-release checks print their valid messages; no Tauri/Node/webview artifact is introduced.

- [ ] **Step 4: Windows-native validation on an isolated library**

Bump `crane-desktop` to `0.3.0`, update the NSIS/build script artifact name, refresh `README.md` (Quick Open, wiki links, inspector, formatting, focus mode, `Ctrl+O`) and add dated rows to `docs/migration/windows-acceptance.md`. Then, against `artifacts/test-data/` preview roots only (never the real library): run `powershell -NoProfile -File scripts/build-windows.ps1`; verify 100%/150%/200% DPI layouts, keyboard-only create→link→Quick Open→inspector→format flow, IME input (Latin, CJK, combining marks, emoji) with exact save/reopen bytes, clipboard both directions, multi-step undo/redo, Narrator names/focus/save-failure announcements, offline restart, 0.2-fixture upgrade on a copy, installer install/upgrade, and rollback to the retained 0.1.0 installer with the library byte-identical. Record Windows version, display scale, and keyboard layout with each result; container or harness passes do not substitute for these rows.

- [ ] **Step 5: Commit**

```bash
git add crates/application/tests/knowledge_perf.rs apps/desktop/Cargo.toml packaging/windows/crane.nsi scripts/build-windows.ps1 README.md docs/migration/windows-acceptance.md Cargo.lock
git commit -m "chore(release): validate Crane 0.3 compatibility and Windows installer"
```

---

## Plan Notes for Executors

- Work task-by-task with `superpowers:executing-plans` (recommended here — see handoff) or `superpowers:subagent-driven-development` if later authorized. Do not start Task *n+1* while Task *n*'s focused suite is red.
- If a step's expected failure differs (e.g. a type already exists), use `superpowers:systematic-debugging`: reproduce, find the root cause, and only then adjust the step — never weaken an assertion to make a suite pass.
- Before any success claim, use `superpowers:verification-before-completion` and re-run the task's command fresh. Release claims additionally require Task 15's physical Windows rows; automated evidence alone is not a release claim.




