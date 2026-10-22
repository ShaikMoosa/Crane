# Crane P0 Nordic Daylight UI Redesign

**Status:** Approved conversational design; written specification awaiting user review
**Date:** 2026-10-22
**Priority:** P0, ahead of public 0.3 release publication and new product features

## Intent and success

Crane will replace its current charcoal-rail, warm-paper, serif-heavy desktop presentation with an original Bear-inspired native writing experience named **Nordic Daylight Archive**. The redesign keeps Crane's identity, features, data, and pure-Rust architecture while adopting the qualities the user selected from Bear as a reference: a quiet three-pane hierarchy, minimal chrome, compact navigation, a highly legible note browser, an editor-first composition, and advanced controls that reveal themselves without crowding the writing surface.

Success means that a writer can open Crane, understand the library state immediately, and begin or resume writing without first interpreting the interface. The active note is visually dominant; navigation is calm but discoverable; Markdown structure appears live while the underlying text remains ordinary editable Markdown.

Bear is a quality and interaction reference only. Crane will not copy Bear source code, branding, icons, wording, screenshots, assets, proprietary themes, or pixel geometry.

## Evidence and decisions

### Verified in the current repository

- Crane is a native Rust application using egui/eframe with wgpu and no webview or Node.js runtime.
- The current desktop has library, note-list, and editor panes; light/dark themes; responsive pane modes; Lucide icons; autosave; formatting; preview; Quick Open; wiki links; an inspector; import/export; backup/restore; and accessibility tests.
- The current editor uses native `egui::TextEdit`, preserving egui-managed selection, IME, clipboard, and undo behavior.
- Markdown parsing already lives in `crane-domain`, which owns `pulldown-cmark` and presentation-neutral preview and knowledge parsing.

### User-approved decisions

- The redesign is P0.
- Replace the current visual system rather than refining it.
- Use the Nordic Daylight Archive direction selected on the visual decision board.
- Use live formatted Markdown.
- Implement live formatting with native `egui::TextEdit` plus a Markdown layouter, not a custom editor or overlay editor.
- Keep the pure-Rust egui/eframe/wgpu stack, local-first behavior, Lucide icon family, existing commands, and persisted formats.

### Inferred constraints

- Light mode is the visual authority and default reference; dark mode must preserve the same hierarchy using cool neutral values.
- Windows is the current shipping platform and receives the physical release gate.
- The writer values visual fidelity and writing flow more highly than adding another feature before release.

## Scope

### Included

- Complete light and dark visual-token replacement.
- Library rail, note browser, editor, formatting controls, inspector, overlays, popups, dialogs, empty states, errors, focus mode, and responsive modes.
- Live Markdown styling inside the native body editor.
- Reduced visual chrome and clearer action hierarchy.
- Accessible interaction states, keyboard behavior, compact targets, and visible save/error feedback.
- Automated UI, parser, cache, and regression coverage plus physical Windows visual acceptance.
- Updating `DESIGN.md` and `.impeccable/design.json` from the finished implementation after the required finish review.

### Excluded

- Accounts, cloud sync, collaboration, billing, attachments, encryption, OCR, and new export formats.
- A custom text-editing engine, hidden Markdown syntax, or full WYSIWYG editing.
- Persistence-schema changes, storage migration, or changes to backup/interchange formats.
- New icon families, remote fonts, remote assets, webviews, JavaScript, or Node.js.
- Copying Bear's brand, themes, assets, proprietary interaction details, or Apple-only platform chrome.

## Chosen visual direction

The durable direction contract lives at `.impeccable/surfaces/apps-desktop-src-app-rs.md`. The approved first-viewport sketch is `.impeccable/mocks/decision/nordic-daylight.html`.

Nordic Daylight uses three contiguous planes:

1. A pale neutral library rail, approximately 208 points wide at the 1440-point reference viewport.
2. A soft-gray note browser, approximately 350 points wide.
3. A bright editor filling the remainder, with a centered 660–700-point writing measure.

The production interface uses a locally available humanist sans for chrome and authored text; egui's licensed fallback remains available. The sketch's serif text is not normative. Crane red is reserved for creation, selection, focus, tags, links, errors, and confirmed save feedback. Pane hierarchy comes from surface value and hairline separators, not shadows or floating cards.

## Semantic design tokens

`theme.rs` will expose semantic roles instead of views embedding literal colors:

- `shell_background`, `rail_background`, `list_background`, `editor_background`
- `surface_hover`, `surface_selected`, `surface_pressed`
- `text_primary`, `text_secondary`, `text_tertiary`, `text_disabled`
- `rule`, `rule_strong`, `focus_ring`
- `accent`, `accent_hover`, `accent_soft`, `on_accent`
- `success`, `warning`, `danger`, `on_status`
- `markdown_marker`, `markdown_heading`, `markdown_emphasis`, `markdown_link`, `markdown_tag`, `markdown_quote`, `markdown_code`, `markdown_code_background`

Light reference values begin near `#F4F5F3`, `#E8EAE7`, `#FFFFFF`, `#202124`, and `#D85645`. Final values may be adjusted to meet contrast and native rendering requirements, but their semantic relationships are fixed. Dark mode uses cool near-black and graphite surfaces; it does not restore the retired warm-charcoal world.

Typography roles are `title`, `heading`, `body`, `interface`, `metadata`, and `monospace`. All ordinary UI and writing roles use the local humanist sans. Monospace is limited to code content. Point sizes remain fixed by role and follow OS scaling; they are not viewport-fluid.

## Shell and responsive layout

### Wide layout

- Rail: 208 points.
- Note browser: 350 points.
- Editor: remaining width, minimum useful writing area preserved.
- Inspector: docks on the right only when doing so leaves the editor's minimum measure intact; otherwise it opens as a right-side sheet.

### Intermediate layout

- The rail collapses to an icon-led strip before the note browser loses useful width.
- Every collapsed destination retains an accessible name and tooltip.
- The note browser remains searchable and shows title, excerpt, time, and limited tag metadata.

### Narrow layout

- Crane presents one explicit pane at a time: library, note browser, or editor.
- Back/navigation actions use standard named controls and preserve the current save-before-navigation gate.
- Common targets remain at least 44 points.

### Focus mode

Focus mode first gives the editor a slight luminance emphasis while surrounding panes dim, then withdraws them in a 150–220 ms transition. With reduced motion enabled, the change is immediate. Exiting restores the prior collection, note, inspector state, and editor focus.

## Library rail

- Crane identity and primary creation remain visible without dominating the rail.
- Collections keep existing semantics and counts: Notes, Today, Untagged, To-dos, Archive, and Trash.
- Nested tags remain segment-aware and visually indented.
- The selected destination uses a white or higher-luminance row plus a thin Crane-red marker; inactive rows do not use accent color.
- Data and Settings sit at the bottom with ordinary labels and Lucide icons.
- Transfer progress and errors remain visible without permanently consuming rail height.

## Note browser

- Collection title, count, search, Quick Open, and new-note action occupy a compact header.
- The new-note action is the only persistent circular accent control.
- Note rows are flat, separated by subtle rules, and sized for dense scanning.
- A row shows title, a maximum two-line excerpt, relative time, and a bounded amount of tag/state metadata.
- Selection uses a brighter surface and narrow accent marker. Hover cannot be the only indication of interactivity.
- Empty results explain the active filter and offer the most relevant recovery action.

## Editor and action hierarchy

- The editor is the brightest and quietest plane.
- The centered writing measure is 660–700 points on wide windows and expands or contracts within safe limits.
- The note title is borderless, strong, and sans-serif; body text is a legible humanist sans with comfortable line height.
- The top toolbar contains pane/focus navigation, preview/edit state, inspector access, and save state.
- Pin, archive, trash/restore, export, theme, and less frequent commands move into clear overflow or context menus while retaining keyboard access and accessible names.
- `Save now` is not a dominant persistent button. Autosave status is quiet when healthy; failure becomes an inline error with Retry.
- The formatting bar remains close to the editor and uses the single Lucide icon vocabulary. Disabled and active states are explicit and never color-only.

## Live formatted Markdown architecture

### Domain representation

`crane-domain` will expose a presentation-neutral highlighter result:

```rust
pub struct MarkdownSpan {
    pub byte_range: std::ops::Range<usize>,
    pub role: MarkdownRole,
}

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
```

The exact public shape may use compact variants or grouped ranges, but it must contain no egui types. It reuses the existing Markdown, wiki-link, and tag grammar so preview, indexing, and editor highlighting do not invent conflicting syntax. Spans are ordered, non-overlapping after composition, UTF-8-boundary safe, and cover malformed or incomplete input without panic.

### Desktop layouter

The desktop owns `MarkdownLayouter`, which converts domain spans into an egui `LayoutJob`:

- The draft `String` remains the only editable source.
- Styling never inserts, removes, replaces, or reorders characters.
- Markdown markers remain visible and editable using `markdown_marker`; markers on the active line may receive slightly stronger contrast.
- Headings change weight and size within bounded values that keep caret metrics stable enough for native editing.
- Tags and safe links use accent text; code uses monospace and a subtle background; quotes use secondary text and a narrow accent rule only where the native layout permits it without changing text.
- Incomplete syntax receives conservative marker/plain styling rather than disappearing.

### Cache

`MarkdownHighlightCache` is keyed by the draft identity and content revision/hash. Parsing runs only when the body changes. Width, scale, or theme changes may rebuild the `LayoutJob` from cached spans without reparsing Markdown. A note switch cannot reuse spans from another draft.

The cache is rebuildable UI state and never persisted. A parser or layout failure degrades to a plain-text `TextEdit` for that frame and records a safe visible status without logging note content.

### Editing guarantees

The implementation must retain native `TextEditState` and the existing stable widget ID per note. It must preserve:

- caret and selection across frames;
- IME composition and Unicode input;
- clipboard operations;
- multi-step undo/redo;
- completion insertion and formatting transformations;
- save-before-navigation behavior;
- autosave timing and failure retention.

## Inspector, overlays, menus, and dialogs

- Quick Open, wiki-link completion, ambiguity chooser, restore confirmation, and action menus inherit the new tokens and spacing.
- The inspector is a right-side sheet at ordinary widths and a docked panel only when editor measure remains acceptable.
- Overlay focus is trapped appropriately, Escape follows the existing dismissal stack, and closing restores editor focus.
- Destructive actions use clear labels and confirmation behavior where already required; the redesign does not weaken safety for visual minimalism.
- Native file and folder dialogs remain platform dialogs and are not restyled.

## Data flow and boundaries

The redesign does not change the command path:

```text
egui intent
  -> existing typed application command/query
  -> domain validation and transition
  -> atomic storage commit
  -> worker response/query refresh
  -> desktop view state and repaint
```

Markdown highlighting is a pure read of the current draft. It does not dispatch commands, perform I/O, mutate the committed knowledge index, or become a source for persistence. Autosave and explicit mutations continue through the existing application layer.

## Failure handling

- Save failure remains visible above the editor body with Retry and the draft intact.
- Highlighting failure falls back to plain text and never blocks editing or saving.
- A missing local preferred font falls back to egui's bundled/system-capable chain without network access.
- Overflow menus and inspector sheets must remain usable at minimum window size and high display scaling.
- Transfer and restore operations retain their current mutation locks, confirmations, and recoverability.
- No note content, link target, imported body, or backup payload is written to diagnostics.

## Accessibility and keyboard contract

- Existing accessible names remain stable unless a new label is clearer and tests are updated intentionally.
- Icon-only controls have accessible names and tooltips.
- Focus indication meets WCAG 2.2 AA contrast and is not encoded by color alone.
- Keyboard traversal follows visible order across rail, browser, editor toolbar, title, body, formatting controls, and inspector.
- `Ctrl+N`, `Ctrl+F`, `Ctrl+O`, Escape behavior, editing shortcuts, and formatting shortcuts remain available.
- High-contrast and reduced-motion settings receive explicit native verification.
- Live styling must not hide characters from assistive technology because the underlying text remains unchanged.

## Performance budget

- Typing must not synchronously rebuild the knowledge index or preview document.
- Markdown span parsing occurs at most once per body revision.
- Reflow may occur on width or theme changes using cached spans.
- Note-list rows remain clipped/virtualized through egui scrolling and do not parse body Markdown for display.
- A near-2 MiB note must remain editable with visible validation and no multi-second UI stall. Performance evidence records timings but does not use brittle wall-clock assertions in ordinary CI.

## Testing strategy

### Domain tests

- Exact UTF-8 byte ranges for headings, emphasis, links, wiki links, tags, lists, tasks, quotes, inline code, and fenced code.
- Incomplete delimiters, escaped markers, CRLF, nested constructs, code suppression, and hostile Unicode never panic.
- Span ordering and non-overlap invariants.

### Desktop unit and harness tests

- Semantic token values and light/dark role relationships.
- Markdown cache invalidation on body changes, note switches, theme changes, and width-only reflow.
- Styling does not alter the draft string.
- Caret, selection, completion, formatting, IME events, clipboard behavior, and undo/redo remain attached to the native `TextEdit` state.
- Wide, collapsed, narrow, inspector-sheet, docked-inspector, focus-mode, empty, loading, save-failure, and transfer-locked layouts.
- Accessible names, focus order, minimum targets, and Escape dismissal.

### Repository gates

Run the existing formatting, workspace tests, Clippy, locked build, dependency-layer guard, native-release guard, and all script self-tests. No web dependency, remote asset, JavaScript source, or Node build hook may appear.

### Native visual acceptance

Capture and inspect the real egui application on Windows in light and dark themes at representative wide, intermediate, and minimum layouts. Repeat at 100%, 150%, and 200% scaling. Verify live IME input, clipboard, multi-step undo/redo, Narrator traversal, keyboard-only operation, focus mode, inspector behavior, long lines, and a near-limit body against an isolated preview library.

## Delivery sequence

1. Introduce semantic theme and typography roles with focused tests.
2. Rebuild the shell, rail, and note browser against the approved composition.
3. Simplify editor chrome and action hierarchy without changing commands.
4. Add domain Markdown spans and the cached native layouter using TDD.
5. Restyle inspector, formatting, overlays, menus, dialogs, empty states, and errors.
6. Refine responsive layouts, focus mode, dark theme, accessibility, and reduced motion.
7. Run the complete automated and physical Windows acceptance gates.
8. Conduct the required finish review, act on its verdict, then regenerate `DESIGN.md` and `.impeccable/design.json` from the shipped interface.

## Acceptance criteria

- The approved Nordic Daylight pane composition is recognizable at the reference viewport and adapts without horizontal clipping.
- The current charcoal/warm-paper visual world no longer appears in the production interface.
- Light and dark themes both preserve rail/list/editor hierarchy and accessible state contrast.
- Markdown displays live role styling while every source character remains editable and persisted unchanged.
- Caret, selection, IME, clipboard, undo/redo, completion, formatting, autosave, and failed-save recovery pass automated and physical checks.
- All existing product workflows remain available and no persistence format changes.
- The full repository gate and native release guard pass.
- Physical Windows evidence is added to `docs/migration/windows-acceptance.md`.
- The finish reviewer returns `ship`, after which DESIGN.md and `.impeccable/design.json` describe the implementation actually delivered.
