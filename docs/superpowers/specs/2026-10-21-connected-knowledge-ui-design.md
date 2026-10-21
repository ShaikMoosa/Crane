# Crane 0.3 connected knowledge and native interface design

Status: Conversational design approved by the user on 2026-10-21. Written-spec review is pending. Implementation planning and implementation remain gated on later user approval.

## Intent and success

Crane 0.3 should make an existing local notebook easier to navigate and calmer to write in. It adds Bear-inspired interaction principles—dense navigation, fast note switching, linked-note discovery, a quiet information panel, and compact formatting—without copying Bear's source, words, branding, screenshots, themes, or assets.

Success means that one offline Windows user can open an existing version-1 library, move between notes from the keyboard, create and follow wiki links, understand a note's structure and inbound references, browse nested tags, and format Markdown without leaving the editor. Existing note IDs, bodies, timestamps, states, tags, imports, exports, backups, and rollback behavior must remain compatible.

The selected approach is a derived knowledge layer. Markdown bodies remain the source of truth. Crane parses them into an in-memory index after loading and after successful mutations. It does not introduce a database, index file, storage migration, webview, JavaScript UI, Node.js toolchain, network dependency, account, or required service.

## Scope

### Included

- Refine the native egui/eframe shell while preserving the design system in `DESIGN.md`.
- Add Quick Open, available from `Ctrl+O` and a visible interface control.
- Recognize wiki links, aliases, and heading targets in Markdown.
- Offer wiki-link completion while editing and safe navigation when reading.
- Show note statistics, table of contents, outgoing links, and backlinks in an information inspector.
- Derive Today, Todos, and Untagged collections from existing note data.
- Present existing slash-delimited tags as a foldable hierarchy with counts.
- Add a compact Markdown formatting bar for syntax Crane can render in 0.3.
- Split the oversized desktop application module into focused controllers as part of the feature work.
- Bundle a reviewed subset of Lucide icons for native, offline rendering.

### Excluded

- Attachments, images as managed files, OCR, sketching, document scanning, or rich-media galleries.
- Encryption, accounts, billing, sync, collaboration, sharing, hosted search, or agent automation.
- Persistent search indexes, SQLite, schema-version changes, or new backup formats.
- Note aliases as stored metadata. A wiki-link alias is display text only.
- Automatic title changes, automatic link rewriting after rename, or automatic merge of duplicate titles.
- Tables, highlighting, footnotes, diagram syntax, or other new Markdown dialect features. The formatting bar only exposes syntax already supported by Crane's editor and preview.
- Copying Bear-specific icons, assets, layout measurements, colors, text, animations, or proprietary behavior.

## Baseline and governing constraints

Crane 0.2 is a released native Rust application. The workspace separates desktop, application, domain, storage, and platform responsibilities. The desktop shell is implemented in egui/eframe with wgpu; version-1 JSON persistence is atomic and tested. The legacy Tauri shell is retired and must not return.

`PRODUCT.md` defines a private, local-first Markdown notebook for a single user. `DESIGN.md` defines the original “writer's desk at dusk” visual system: charcoal library rail, warm-paper note list, bright editor canvas, serif authored content, compact sans-serif controls, and restrained ember accents. Those documents outrank the external reference product.

The existing save contract remains authoritative: the desktop owns the draft, a typed application command persists it, and committed state changes only after an atomic write succeeds. Derived knowledge must follow that same commit boundary.

## Experience and information architecture

### Wide layout

The resting wide layout remains three contiguous panes:

1. A 220–236 point library rail.
2. A 310–330 point note list.
3. A flexible editor with a practical minimum width.

The rail contains the create action, primary collections, and hierarchical tags. The note list contains a collection heading, result count, search, compact sort/filter affordances, and flat note rows. The editor contains sparse top controls, the title and body, quiet save state, and a compact formatting bar near the lower edge.

The information inspector is transient rather than a permanent fourth pane. When the editor has at least 760 points available, a 320-point inspector docks inside the editor region and the writing column reflows. Below that editor width, the same inspector floats over the right side of the editor and closes before pane navigation. It has four tabs: Statistics, Outline, Links, and Backlinks. The selected tab is remembered only for the current process.

### Responsive layout

- Above 900 points: full rail, full note list, and editor.
- From 681 through 900 points: 72-point collapsed rail, 290-point list, and editor.
- At 680 points and below in the UI harness: one named pane at a time—library, list, or editor—with explicit back navigation.
- Focus mode hides both navigation panes and gives the editor the window. Leaving focus mode restores the previous pane widths and selection.
- The inspector overlays rather than forcing a fourth column at compact sizes.
- Common targets are at least 40 points on desktop and 44 points in compact mode.

### Visual behavior

Pane hierarchy comes from material color and one-pixel separators, not floating cards. Note rows remain rectangular. The selected note receives a thin ember marker and a restrained tonal fill. Ember continues to indicate evidence—selection, focus, an active tool, a tag, or confirmed save—not decoration.

The note list remains dense enough to scan. A row shows title, a short excerpt, modified time, and only the tag/status metadata that fits without wrapping the row into a card. The editor removes the current visually heavy action row in favor of quiet icon controls, an information control, and the formatting bar.

## Derived collections and tag navigation

Collections do not add fields to `Note`:

- **Notes:** active, non-trashed, non-archived notes.
- **Pinned:** active notes whose existing `pinned` field is true.
- **Today:** active notes whose `updated_at` falls on the current local calendar date.
- **Todos:** active notes containing at least one unchecked Markdown task.
- **Untagged:** active notes with no derived tags.
- **Archive:** notes whose existing `archived` field is true and `trashed` is false.
- **Trash:** notes whose existing `trashed` field is true.

Tag hierarchy is a view over existing slash-delimited tags. For example, `project/crane/ui` contributes the nodes `project`, `project/crane`, and `project/crane/ui`. Parent counts are the number of distinct visible notes under that branch, not a sum that double-counts a note carrying multiple descendant tags. Selecting a parent includes all descendants. Fold state is session-only in 0.3. Tag text and persistence remain unchanged.

## Wiki-link grammar and resolution

Crane recognizes these forms outside code spans and fenced code blocks:

- `[[Note title]]`
- `[[Note title|Visible alias]]`
- `[[Note title#Heading]]`
- `[[Note title#Heading|Visible alias]]`

The first unescaped `|` separates the display alias. The first unescaped `#` before the alias separates the heading target. Leading and trailing whitespace around the target title, heading, and alias is ignored for resolution and display fallback. An empty heading means there is no heading target; an empty alias falls back to the target title. Empty titles, unmatched brackets, nested `[[` sequences, and escaped opening delimiters such as `\[[` remain ordinary Markdown. The parser is total: malformed input produces text, not a save error.

Title lookup normalizes by trimming Unicode whitespace, lowercasing with Rust's Unicode lowercase conversion, and collapsing runs of whitespace to one ASCII space. It does not remove punctuation. Heading lookup uses the same normalization, additionally replacing whitespace runs with `-` and ignoring leading Markdown heading markers. Duplicate headings receive occurrence suffixes `-2`, `-3`, and so on within one note.

Resolution has three explicit outcomes:

1. One normalized title match opens that note.
2. No match offers to create a note using the trimmed target title.
3. Multiple matches show a chooser containing title, excerpt, and modified time; Crane never picks arbitrarily.

When a note resolves but its heading does not, Crane opens the note at the top and shows a non-destructive “Heading not found” notice. Link creation does not add persisted alias metadata. Renaming a note can therefore create unresolved incoming links; the Backlinks/Links views expose that state rather than silently rewriting user text.

## Knowledge model and index

Add a pure domain knowledge subsystem under `crates/domain/src/knowledge/`:

- `parser.rs` produces facts for one note from Markdown.
- `index.rs` owns cross-note lookup, inbound links, ambiguity, and incremental replacement.
- `statistics.rs` calculates note measurements.
- `tags.rs` creates hierarchical tag projections.

The central per-note value contains the note ID and committed revision inputs, normalized title, headings with UTF-8 byte ranges, outgoing wiki links with UTF-8 byte ranges, task counts, derived tags, and statistics. Statistics count Unicode scalar values as characters, whitespace-delimited runs as words, Markdown paragraph nodes as paragraphs, and Markdown heading/task nodes for their respective totals. Reading time uses 200 words per minute and rounds any non-empty result up to one minute.

`KnowledgeIndex` maintains:

- normalized title to one or more note IDs;
- note ID to parsed facts;
- target key to inbound link references;
- hierarchical tag nodes and distinct-note counts;
- searchable normalized title, tags, and body text needed by Quick Open.

Build the full index after a library opens. After a successful note create or body/title save, replace that note's parsed entries. After a successful pin/archive/trash/restore state change, update only its collection visibility metadata. Rebuild after backup restore, batch import, or any operation that replaces the committed store. Archive and trash state affect visible collections and search filtering but do not erase knowledge facts, so restoring a note does not require reparsing its unchanged body.

The index is memory-only. Closing and reopening deterministically reconstructs it. It is never serialized into the library or backup. A failed persistence operation leaves both committed store state and the derived index unchanged.

## Application services and data flow

The application layer exposes typed read models rather than giving the desktop access to domain internals or storage:

- `quick_open(query, scope)`
- `resolve_wiki_link(source_note_id, link)`
- `complete_wiki_link(fragment)`
- `knowledge_for(note_id)`
- `tag_tree(scope)`
- `notes_for_collection(collection)`

Quick Open defaults to active notes and includes archived notes only through an explicit scope toggle. Trash is excluded. Blank queries return recent active notes, with pinned notes first. Non-blank results rank exact normalized title, title prefix, title substring, tag match, then body substring; ties use pinned state, descending `updated_at`, and stable note ID. The initial implementation is deterministic substring ranking, not speculative fuzzy matching.

The mutation flow is:

```text
editor draft -> typed application command -> validate candidate store
             -> atomic persistence -> publish committed store
             -> replace/rebuild derived index -> publish view models
```

The index update occurs on the same serialized application worker after persistence and before the success result is published. Index construction must not perform filesystem, network, UI, identity, or billing work. Query results contain safe snippets and identifiers, not capabilities to read arbitrary files.

## Desktop components

The feature work should reduce `apps/desktop/src/app.rs` by extracting focused modules while retaining one top-level composition owner:

- `sidebar.rs`: collections, hierarchical tags, fold state, and responsive rail.
- `quick_open.rs`: overlay state, query input, result selection, keyboard handling, and focus restoration.
- `knowledge_navigation.rs`: link completion, link activation, ambiguity chooser, and heading navigation.
- `inspector.rs`: Statistics, Outline, Links, and Backlinks views.
- `formatting.rs`: selection-aware Markdown transformations and toolbar presentation.
- `icons.rs`: the bounded Lucide catalog and accessible icon-button helpers.

These controllers receive typed application results and emit typed intents. They do not parse the library, persist files, run shell commands, or reach into storage adapters.

### Quick Open

`Ctrl+O` opens a centered transient overlay and focuses its search field. Arrow keys change the active result, Enter opens it, and Escape closes the overlay and restores the previous editor focus. Pointer selection behaves identically. Opening another note follows the existing draft-save rule; a failed required save keeps the current draft selected and leaves an actionable error visible.

### Wiki-link completion and navigation

Typing an unescaped `[[` opens completion near the text cursor. Results are limited to a small visible set with keyboard scrolling. Enter inserts the selected title and closing brackets; Escape dismisses without changing text. Typing `#` after a resolvable title switches completion to that note's headings. Typing `|` edits display text without changing resolution.

Because the current editor is a source editor, edit mode shows wiki links as Markdown text. Link activation occurs in preview and the inspector. Adding direct modifier-click navigation inside the source editor is out of scope unless egui exposes it without breaking normal text selection, IME, or undo behavior.

### Inspector

- **Statistics:** word, character, paragraph, heading, task, completed-task, and reading-time values.
- **Outline:** headings in document order, indented by level; activation moves the editor or preview to the heading.
- **Links:** outgoing links with resolution state and heading/alias context.
- **Backlinks:** source note title and a short, safely derived context excerpt around each inbound link.

The inspector represents empty, unresolved, and ambiguous states explicitly. It does not hide malformed syntax or claim a backlink when resolution is ambiguous.

### Formatting bar

The 0.3 bar exposes heading level, task, bulleted list, numbered list, quote, bold, italic, inline code, code block, and link insertion. Actions wrap a selection when possible, insert paired markers at an empty selection, or apply line prefixes to every selected line. Repeating a compatible action unwraps or removes the prefix when the selection is structurally exact. Transformations preserve the nearest valid caret/selection and participate in the editor's normal undo history.

Tables, highlighting, and media insertion remain excluded because Crane does not yet render or manage them. Their Lucide candidates are not bundled until the corresponding feature is approved.

## Lucide icon policy

Crane will vendor only the icons referenced by shipped controls from the official Lucide v1.60.0 release. Assets live under `apps/desktop/assets/icons/lucide/`; the upstream version and original filenames are recorded in an adjacent manifest. The root `THIRD_PARTY_NOTICES.md` includes the Lucide ISC notice and the MIT notice for any Feather-derived icons used.

The application does not fetch icons at build time or runtime and does not add a JavaScript package. The implementation plan must select a renderer compatible with Crane's pinned egui/eframe 0.37 stack and prove it in a small test before adopting it. The current `egui-lucide` 0.1 crate is not selected because its documented dependency targets egui 0.36. A compatible `egui_extras` SVG loader or a small native painter adapter may be used only after dependency and binary-cost review.

Icons use Crane colors, 16–18 point artwork, and an approximately 1.75 point stroke where the renderer permits. Icon-only controls have accessible names, tooltips, visible keyboard focus, and 38–40 point desktop hit targets. Important destructive or ambiguous operations retain text labels. Icon color alone never communicates state.

The initial shipped subset should cover navigation, disclosure, search, creation, information, pane visibility, formatting, link state, success, progress, and warnings. The exact manifest follows the controls that survive implementation; unused candidate assets are not committed.

## Failure handling and recovery

- Malformed or incomplete wiki syntax remains editable text and never blocks persistence.
- Knowledge parsing is total over arbitrary UTF-8 note text. Malformed constructs become ordinary text rather than a recoverable parser error or an empty note view.
- A save failure retains the draft, exposes Retry, and blocks navigation or close paths that would discard newer text.
- Index updates are computed before publishing the successful view state. If an internal consistency check detects stale cross-note entries, Crane invalidates and fully rebuilds the memory-only index while retaining the committed note and editable draft.
- Missing headings open the resolved note at its top with a quiet notice.
- Ambiguous titles require a user choice and are never resolved using list order.
- Quick Open, completion, and inspector queries cancel or ignore stale results when their query revision or selected note changes.
- A missing vendored icon is a compile/test failure. Runtime controls never disappear silently because of an asset path error.
- Error messages do not log note bodies or expose more of a local path than the existing transfer diagnostics require.

## Accessibility and keyboard contract

All operations available through a Lucide icon also have an accessible name. Frequently used actions have tooltips and shortcuts where appropriate. Focus order follows rail, list, editor chrome, title, body, formatting bar, and inspector. Opening an overlay or chooser moves focus into it; closing restores the initiating control or editor position.

Required shortcuts are:

- `Ctrl+O`: Quick Open.
- `Ctrl+N`: new note, unchanged.
- `Ctrl+F`: focus the note-list search field, preserving Crane's current global library-search behavior.
- `Escape`: close the topmost transient surface without discarding a draft.
- Arrow keys and Enter: navigate and choose Quick Open, completion, inspector links, and ambiguity results.

Visible focus, WCAG 2.2 AA target contrast, reduced-motion behavior, screen-reader names through AccessKit, DPI scaling, clipboard, IME, selection, and undo/redo remain release requirements. Routine editing receives no decorative animation.

## Performance and cache behavior

Parsing and cross-note lookup run outside repeated paint work. Per-note facts are cached by note ID plus the committed fields that affect them. The editor may separately cache draft outline/statistics by draft revision so the inspector can reflect unsaved text without publishing backlinks from uncommitted content.

Initial indexing is linear in total note text plus extracted facts. Incremental save work is linear in the changed note plus its affected inbound/outbound entries. Quick Open uses the existing in-memory corpus and deterministic ranking; no on-disk index is introduced. UI result lists are bounded and virtualized or clipped so rendering cost follows visible rows rather than library size.

A synthetic large-library fixture will exercise at least 5,000 notes with mixed links and tags. Timing is recorded as diagnostic evidence, not a flaky wall-clock unit-test assertion. Acceptance requires that indexing does not block frame painting after the initial library handoff and that incremental typing does not trigger full-library reparsing.

## Testing strategy

### Domain

Add table-driven tests for all supported link forms, escaping, malformed delimiters, aliases, heading targets, duplicate headings, punctuation, Unicode lowercase behavior, inline/fenced code exclusion, source ranges, tasks, statistics, and nested tag counts. Property-style cases should establish that arbitrary UTF-8 input does not panic and that parsing does not modify source text.

### Application

Test complete index construction, incremental replacement, title rename consequences, note creation, pin/archive/trash/restore visibility, backup-restore/import rebuilds, stale-query suppression, unresolved and ambiguous resolution, deterministic ranking, and failure before/after persistence. Existing failed-save recovery tests remain mandatory.

### Desktop

Use `egui_kittest` for Quick Open keyboard operation, overlay focus restoration, completion insertion, ambiguity selection, inspector tabs and empty states, formatting transformations, pane collapse, focus mode, accessible names, and unsaved-draft navigation protection. Screenshot/golden coverage should include wide, compact, and one-pane states in both themes, but screenshots do not replace behavior assertions.

### Compatibility and release

Re-run version-1 library and backup fixtures unchanged. Verify an upgrade from Crane 0.2 on an isolated copy, offline startup, native rendering, Windows DPI, keyboard-only navigation, IME, clipboard, installer behavior, and retained rollback procedure. Containers may run headless Rust checks but do not prove native GUI, GPU, dialog, accessibility, upgrade, or installer behavior.

The release gate includes:

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --all-targets --all-features --locked
powershell -NoProfile -File scripts/check-layers.ps1
powershell -NoProfile -File scripts/check-layers.ps1 -SelfTest
powershell -NoProfile -File scripts/check-native-release.ps1
powershell -NoProfile -File scripts/check-native-release.ps1 -SelfTest
```

## Delivery sequence and exit gates

This is one release delivered through independently reviewable stages:

1. **Knowledge grammar and facts.** Exit when pure parsing, statistics, task, heading, and tag fixtures pass without changing persisted models.
2. **Knowledge index and application queries.** Exit when complete/incremental indexing, deterministic Quick Open, backlinks, ambiguity, and mutation-boundary tests pass.
3. **Desktop module extraction and icons.** Exit when controllers replace the relevant responsibilities in `app.rs`, the approved Lucide subset renders offline, licensing is present, and existing UI behavior still passes.
4. **Navigation and inspector.** Exit when Quick Open, completion, link resolution, heading navigation, all inspector tabs, focus behavior, and failed-save protection pass.
5. **Formatting and responsive refinement.** Exit when selection-aware transformations, three responsive states, focus mode, themes, accessible labels, and visual-regression evidence pass.
6. **Compatibility and native release validation.** Exit only after the complete repository gate, isolated 0.2 upgrade, Windows interaction checks, installer validation, and rollback evidence pass.

Each implementation task follows test-driven development: add and observe the failing behavior test before production code, then run the narrow test and the appropriate broader gate. Unexpected results use systematic debugging before code changes. No release claim is made from container-only evidence.

## Acceptance criteria

Crane 0.3 is acceptable when:

- An unchanged version-1 library opens and round-trips without schema or backup changes.
- Quick Open, wiki-link completion, link navigation, backlinks, outline, statistics, derived collections, hierarchical tags, and the approved formatting actions work by keyboard and pointer.
- Duplicate titles, unresolved links, missing headings, malformed syntax, stale results, and failed saves have deterministic, non-destructive behavior.
- The interface preserves Crane's three-material design and works in wide, compact, one-pane, focus-mode, light, and dark states.
- Every shipped icon is local, licensed, pinned to its source version, accessible, and actually used.
- No Tauri, Electron, webview, JavaScript/TypeScript UI, Node dependency, remote script, remote font, remote icon, account, sync service, or required server is introduced.
- Layer and native-release checks pass, and Windows-native acceptance evidence is recorded.

## Sources

- Local product authority: `PRODUCT.md`, `DESIGN.md`, `AGENTS.md`, current workspace manifests, domain models/queries/preview code, application commands, and desktop shell.
- [Bear](https://bear.app/) as an interaction reference only.
- [Bear note linking](https://bear.app/faq/how-to-link-notes-together/) for public descriptions of wiki-link workflows.
- [Bear information panel](https://bear.app/faq/how-to-use-the-info-panel-table-of-contents-and-backlinks-in-bear/) for public descriptions of statistics, outline, and backlinks.
- [Bear search](https://bear.app/faq/how-to-search-notes-in-bear/) for public descriptions of Quick Open and search behavior.
- [Bear sidebar](https://bear.app/faq/about-the-sidebar-in-bear/) and [Bear tags](https://bear.app/faq/how-to-use-tags-in-bear/) for public navigation and nested-tag concepts.
- [Bear Markdown formatting](https://bear.app/faq/how-to-use-markdown-in-bear/) for public toolbar concepts.
- [Lucide](https://lucide.dev/) and the [Lucide license](https://lucide.dev/license) for the selected open icon source and licensing terms.

## Review state

The conversational design, including the native interface direction, derived knowledge architecture, failure behavior, verification strategy, and Lucide use, was approved on 2026-10-21. This written specification has been checked for placeholders, internal contradictions, ambiguous resolution rules, and accidental scope expansion. The next gate is explicit user review of this file. Only after approval may `superpowers:writing-plans` create the detailed implementation plan.
