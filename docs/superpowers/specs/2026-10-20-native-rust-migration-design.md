# Crane native Rust migration design

Status: Approved by the user on 2026-10-20 after written-spec review. The user selected a staged migration preserving existing features. Implementation and runtime cutover still depend on review of the detailed implementation plan and completion of its gates.

## Intent and success

Move Crane from Rust/Tauri with a static JavaScript interface to the user's selected `saas` architecture: Rust, egui/eframe, and wgpu. Preserve Crane as a private, offline Markdown notebook for one person on Windows. The result must open the existing library without changing note identity or losing content, support the current writing and transfer workflows, and install on this machine without WebView2.

This is an interface and application-boundary migration, not a new product. Accounts, billing, sync, collaboration, encryption, attachments, MCP, remote control endpoints, and hosted services are outside its scope. Docker is optional for later headless checks and is not required for this migration or for running Crane.

The chosen approach is staged extraction and replacement. A full rewrite would discard useful Rust behavior and require rediscovering its safety rules. Maintaining two independent products would duplicate storage and feature maintenance. The transitional Tauri shell is retained only until the native replacement passes the cutover gates.

## Evidence and present state

Inspected baseline: commit `6770f1292f5d741cedffff3ec99c84221b6a179f`.

- **Verified in source:** `src-tauri/src/model.rs` defines schema version 1, stable string IDs, title/body/tags, millisecond timestamps, and pinned/archived/trashed flags. Serialization uses camelCase.
- **Verified in source:** `src-tauri/src/persistence.rs` writes an exclusively created temporary sibling, flushes it, and replaces the destination. The Windows implementation uses `MoveFileExW`.
- **Verified in source:** Markdown parsing, export manifests, and backup validation/transactions are implemented in Rust under `src-tauri/src/transfer/`. Their current files mix pure transformations with filesystem work and need a responsibility split, not wholesale reuse in domain code.
- **Verified in source:** Tauri `main.rs` owns startup, commands, clocks/IDs, dialogs, reads, and application state. It registers four note commands and six transfer commands. The note mutation helper changes memory before persisting; import and restore already use candidate-state commits.
- **Verified in source:** `ui/core.mjs` owns filtering, ordering, excerpts, and Markdown rendering. `ui/app.js` owns draft state, a 550 ms save delay, selection, keyboard shortcuts, theme selection, and transfer controls. These behaviors must move into Rust.
- **Verified in source:** startup resolves Tauri `app_data_dir()` with identifier `app.crane.desktop`, then opens `notes.json`. The installed Tauri resolver uses the operating system data directory plus that identifier. The native replacement must match this resolution; a new eframe default path must not silently create an apparently empty library.
- **Verified in source:** browser preview uses `crane.store.v1` and `crane.theme` in browser localStorage. That is a separate data location from the native library.
- **Verified in repository:** no tracked root Cargo workspace, standalone native packaging recipe, or CI workflow exists at the baseline. Tauri configuration currently drives Windows delivery.
- **Previously verified in this chat:** 30 JavaScript tests, 31 Rust tests, formatting, strict Clippy, a development build, and an NSIS installer passed before this planning task. They have not been rerun for this document, and they do not establish egui readiness.
- **Verified locally:** Rust/Cargo 1.99.1 are installed. The current manifest's declared 1.77.2 minimum is not evidence that the new dependency graph supports that minimum.
- **Documented upstream:** eframe integrates egui with wgpu, platform accessibility, fonts, and clipboard support; rfd provides native dialogs. These facilities still require Windows integration tests in Crane. Sources are listed below.

Representative current save flow: body/title input changes a JavaScript draft, the 550 ms timer calls `bridge.saveNote`, Tauri `save_note` validates and rederives tags, `with_store` mutates the store and persists it, and the returned note refreshes the list and save label. The new flow retains drafts until a successful durable result and prevents an older result from replacing newer typing.

## Target boundaries

Create one Cargo workspace with these members only:

- `apps/desktop` (`crane-desktop`): egui/eframe entrypoint, wgpu configuration, view state, draft editing, UI task/result handling, panels, and accessibility labels. Its release binary is `crane.exe`.
- `crates/domain` (`crane-domain`): note/store value types, invariants, tag extraction, query/filter/sort rules, and pure Markdown/backup data transformations.
- `crates/application` (`crane-application`): typed commands and queries, transaction orchestration, save/transfer state, restore confirmation lifecycle, and port traits. It depends on domain, not on desktop, storage, or platform implementations.
- `crates/storage` (`crane-storage`): versioned JSON compatibility, bounded reads, atomic writes, safety backups, and import/export filesystem transactions. It implements application storage ports and uses platform helpers where OS behavior differs.
- `crates/platform` (`crane-platform`): application-data directory resolution, native dialogs, Windows atomic replacement, local font loading, and single-instance safeguards. It implements narrow application ports and never depends on storage or desktop.

The desktop composes application, storage, and platform. Domain has no UI, OS, filesystem, clock, HTTP, identity, or billing dependencies. Application receives time, IDs, persistence, and selected-file capabilities through ports. No application command accepts an arbitrary shell operation. A path needed inside a storage adapter is not exposed as a general read/write command.

No CLI, server, protocol, sync, automation, or testkit crate is added speculatively. Fixtures and fakes live beside the crates that use them. Packaging scripts and dependency-boundary checks may live under `scripts/` and `packaging/windows/`; they do not introduce a Node dependency.

Keep `src-tauri/` as an explicitly excluded legacy package while the new workspace is built. During extraction, the existing shell calls the shared Rust application/core rather than acquiring a second independent implementation. Remove this adapter and the JavaScript tree only at the final cutover milestone.

## Commands, tasks, and draft ownership

Replace string IPC and DOM callbacks with typed Rust operations for opening/querying the library, creating/saving notes, setting note state, Markdown import/export, backup creation/inspection, and confirmed replacement. Pin state changes to an enum instead of a free-form field name. Return stable error codes with safe user messages and structured completed/cancelled outcomes.

One application worker serializes library mutations. The desktop owns editable drafts; the worker owns the committed library. Disk work and transfer parsing run away from the egui frame callback. A bounded request channel and result channel connect them, and completion requests a repaint. Queue saturation is visible; mutations and drafts are not silently dropped.

Each save carries a note ID and monotonically increasing draft revision. A result acknowledges that exact revision. A newer draft remains dirty, is never overwritten by an older result, and is scheduled for the next save. Coalesce unsent saves for the same note; do not queue a full note on every keystroke. Switching notes, starting a transfer, and closing the window wait for the latest required save. A failed save retains the draft and exposes Retry. A normal close is postponed while the save is pending or failed; an operating-system forced termination is outside the durability promise for unsaved input.

For every mutation, derive and validate a candidate store, persist it atomically, and only then publish the committed state. Failed persistence leaves the previous committed library in memory and on disk. No GUI success event is emitted before persistence succeeds.

Transfers allow only one active operation. Reject conflicting mutations throughout import or replacement, retain readable note content, and restore focus when finished. Silent cancellation is valid before the commit boundary. Once replacement starts, disable Cancel and Escape until the operation returns.

Restore inspection creates a single-use confirmation object bound to the inspected contents and current library revision. Revoke it on cancellation, replacement inspection, successful consumption, or exit; expire it after five minutes using monotonic time. A changed library requires reinspection. Invalid, expired, or reused confirmations never write data.

## Existing data, startup, and rollback

Keep `schemaVersion: 1` and `backupVersion: 1` for the migration. Preserve IDs, Unicode text, exact note bodies, timestamps, tag arrays, flags, and note order when reading and saving an unchanged library. Keep the existing validation boundaries: 500 UTF-8 bytes per title, 2 MiB per body, timestamps at most `8_640_000_000_000_000`, unique non-empty IDs, 64 MiB aggregate Markdown selection, and 256 MiB per backup input/output. Unit labels in user messages must accurately describe these byte limits.

Retain unknown JSON fields in store, note, and backup envelopes when round-tripping supported version-1 data. Unsupported versions fail visibly. Do not reinterpret unknown fields as authorization, file paths, or commands. Existing valid data is never reseeded because parsing or access fails; onboarding seeds are created only when the selected library is genuinely absent.

Native preview builds use an isolated test data root and cannot silently fall back to the production root. Tests use synthetic fixtures, including an empty library and notes in every state; do not read the user's real note bodies for fixtures. The old installed app remains usable during development.

Before native cutover, resolve and display the existing library location, ensure the legacy application is closed, validate a read-only snapshot, and create a uniquely named, restorable pre-migration backup. If that backup cannot be created, validated, or written within the size cap, cutover stops. Only after this gate does the new app use the existing data directory. On Windows the expected layout is the roaming app-data root plus `app.crane.desktop/notes.json`; resolver contract tests must verify it against the legacy resolver on this machine.

The native app holds an exclusive instance/library guard. Because the old binary does not participate in a new lock protocol, the upgrade must refuse cutover while the legacy process is running and must not claim safe simultaneous old/new writes. During development the two shells use separate data roots. The release upgrade replaces the old installed binary rather than presenting two production writers.

Rollback closes the native app, reinstalls the retained old installer, and opens the still-compatible version-1 store. The pre-migration backup is an additional recovery option, not a file automatically restored over newer notes. If content recovery is required, obtain explicit replacement confirmation and explain that edits newer than the selected backup would be replaced. Uninstalling either version must preserve notes, backups, and native settings.

Browser-preview data is separate. Before the preview is retired, document a user-triggered export path for anyone who used it; do not scrape webview/browser storage. Native settings move to a small versioned Rust settings file. The old localStorage theme choice may need to be selected once in the new interface; it is not part of the note schema.

## Functional and interface parity

Retain create/autosave, search across title/body/tags, nested inline tags, pinning, archive, recoverable trash, Markdown source editing and reading preview, word/date metadata, light/dark themes, keyboard shortcuts, and every shipped transfer operation. Permanent deletion remains outside scope.

Port query/formatting behavior into tested Rust functions rather than translating HTML strings into egui. Rust tag extraction is authoritative; temporary JavaScript draft tags must not define a second persisted syntax. Preserve tags on unchanged existing notes and rederive them on deliberate body edits/import. Add fixtures for the places where the current JavaScript and Rust extractors disagree, including punctuation, Unicode, nested segments, and a hyphenated tag.

Use a Rust Markdown parser and egui-native rendering of headings, paragraphs, emphasis, fenced code, quotes, lists, and read-only task checkboxes. Raw HTML is displayed inertly; images never trigger remote fetches. External link activation is explicit and restricted to HTTP/HTTPS through a platform adapter. The reader and editor must not block every frame on full-library filtering or Markdown parsing; cache by committed/draft revision and query inputs.

Preserve the design intent in `DESIGN.md`: contiguous charcoal library rail, warm-paper list, editorial canvas, restrained ember, compact controls, visible focus, and serif content versus sans-serif controls. CSS is reference material, not a target dependency. Recreate labels, menus, modals, and layout using native egui widgets and AccessKit information. Prefer built-in accessible widgets over painted interactive regions.

Use local system fonts when permitted and available, with licensed bundled fallback coverage; never redistribute Windows font files without a license or download fonts at runtime. The new renderer requires its own acceptance checks for Unicode, selection, undo/redo, clipboard, IME composition, long lines, scrolling, zoom/DPI, and screen readers. Browser spellcheck is not automatically inherited: validate whether a supported local Windows adapter can preserve it, and surface any proposed omission for approval before claiming feature parity.

Preserve the initial 1240 by 780 window and existing 720 by 560 minimum in logical points, subject to a native DPI check. Retain the 900-point rail-collapse behavior. Test the design's one-pane mode below 680 points in the UI harness; if making that mode reachable requires changing the desktop minimum, record and approve that small product change instead of silently changing the constraint.

Support `Ctrl+N`, `Ctrl+F`, Tab/Shift+Tab traversal, normal editing shortcuts, actionable failure status, and modal focus restoration. Use 40-point common desktop targets and 44-point compact targets. Translate the successful-save cue without continuously repainting an idle window; honor reduced motion. Screenshots alone do not prove keyboard or screen-reader behavior.

## File transfers and trust boundaries

Reuse the existing Markdown and backup semantics: all-or-nothing UTF-8 `.md` import creates new active notes; a selected trashed note may be exported explicitly; library export writes `Notes/` and `Archive/` and excludes trash; complete backup includes every state; confirmed restore writes and flushes the current store's safety backup before replacing it.

Keep readers bounded while reading, not only after allocation. Limit individual Markdown reads to the remaining batch budget, then validate parsed note limits. Validate normalized save destinations before overwrite confirmation. Reserve every temporary sibling/file exclusively, publish an export directory only after all entries succeed, and clean up only artifacts created by that operation. Use filesystem-equivalent collision handling on Windows, including Unicode case collisions and reserved superscript device names; exclusive creation is the final safeguard against accidental overwrite.

User-selected file/folder access is a capability obtained from a native dialog, not a global filesystem service. Refuse symlink/reparse-point leaves for destructive writes and test the selected ancestor policy on Windows. Backup/restore messages identify the failed phase without logging note contents or unnecessarily exposing sensitive paths. Do not add an HTTP server, inspection listener, or agent endpoint to deliver desktop testing.

## Stages and exit gates

1. **Compatibility and shared core.** Add the Cargo workspace, synthetic version-1 fixtures, extracted domain/storage code, application ports/commands, and a thin legacy adapter. Exit when both the legacy application and the shared Rust core pass existing regression coverage plus failed-save, unknown-field, and legacy-format round trips. The old installation still works.
2. **Native writing workflow.** Build the egui/eframe + wgpu shell against isolated data; port queries, drafts/autosave, note states, navigation, themes, and preview. Exit when create/edit/reopen, pending-save failures, rapid typing with delayed results, keyboard operation, IME/clipboard, DPI, and GPU initialization pass on Windows.
3. **Transfers and recovery parity.** Connect native file dialogs and the shared transfer handlers; implement inspection/revocation/expiry and safe modal behavior. Exit when round trips, archive/trash behavior, cancellation, malformed/oversized inputs, export collisions, failed safety backups, and failed replacement all pass without corrupting the previous library.
4. **Delivery and cutover.** Build an independent Windows installer, test a real upgrade/reinstall/uninstall cycle on an isolated Windows profile, validate the legacy path and rollback, and run the full release gate. Only then make the native binary the default and retire `ui/`, JavaScript tests, Tauri code/configuration/plugins/build dependencies, and the old Tauri packaging path.

These are sequential milestones in one migration. The implementation plan should give each independently testable task a failing-behavior test where appropriate, precise files/interfaces, verification, and a commit. Preserve the user's prior native/inline execution preference unless they change it.

## Toolchain, delivery, and documentation

Use the installed Rust 1.99.1 toolchain as the initial tested toolchain, recorded in a root toolchain file when implementation begins. Select mutually compatible published egui/eframe/wgpu versions, prove the Windows build, and lock them through the root `Cargo.lock`; do not copy an unrelated repository's versions or claim an untested MSRV. AccessKit and OS clipboard support are required. Do not enable optional network inspection, web targets, or unnecessary rendering backends.

Use rfd directly for native dialogs; version 0.16.0 is already present transitively in the legacy lockfile and is the first reuse candidate. Check its direct feature choices and the selected egui stack together before locking. Keep native C/OS dependencies confined to platform/rendering/packaging roles with their distribution cost documented. Pure Rust describes Crane's application implementation; it does not eliminate the Windows SDK or graphics driver.

Retain NSIS as the Windows installer format through a standalone, repository-owned packaging recipe using the installed native toolchain. Preserve product name, `crane.exe`, app identifier, icon ownership, install scope, upgrade detection, and uninstall-data behavior after inspecting the actual legacy installer metadata. Choose a version greater than 0.1.0 for native cutover. Compile metadata, dependencies, expected artifacts, and checksums must agree. Sign only when credentials and signing scope are supplied; do not make an unsigned development installer appear signed.

Add a Windows CI job for locked workspace checks and an installer build. GUI/GPU/dialog/upgrade smoke tests run on a suitable Windows machine; ordinary hosted headless CI cannot stand in for interactive checks. A container, if later added, tests only the headless core and follows the `saas` Docker reference. macOS/Linux delivery needs its own validation and is not claimed by this migration.

`AGENTS.md` records the chosen target and temporary legacy rules now. During implementation update `PRODUCT.md`, `README.md`, and `DESIGN.md` at the stages their claims become true. Preserve dated historical plans/specifications, marking them superseded by the new implementation rather than rewriting their history. Add only the architecture, compatibility, testing, and packaging documents needed for this migration; avoid a speculative SaaS documentation tree.

## Verification and acceptance

While the legacy adapter remains, run its JavaScript suite and locked Rust format/test/Clippy/build commands. The root workspace must also pass:

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build --workspace --all-targets --all-features --locked
```

Enforce inward crate dependencies and absence of Tauri/webview/JS/Node from the final build through dependency metadata and tracked-file checks. Keep historical documentation references allowed; do not mistake a text search for a dependency audit. Port behavior tests from JavaScript to Rust before removing them; an equivalent Rust fixture/test must exist for every retained product behavior, while DOM/IPC-specific tests may retire with that interface.

Release acceptance additionally requires unchanged version-1 note/backup round trips, injected failed-write/restore recovery, copy/paste and undo/IME checks, keyboard/screen-reader operation, offline native launch/reopen, native open/save/folder dialogs, and Windows install/upgrade/uninstall/rollback tests. Record exact commands and manual evidence separately from unexecuted checklist items. Performance, binary-size, build-time, and accessibility improvements are hypotheses until measured on this machine.

## Sources

- Local baseline files: `src-tauri/src/{main,model,persistence}.rs`, `src-tauri/src/transfer/`, `src-tauri/{Cargo.toml,Cargo.lock,tauri.conf.json}`, `ui/{app.js,bridge.mjs,core.mjs,transfer-controller.mjs}`, `tests/`, `PRODUCT.md`, and `DESIGN.md`.
- Local installed Tauri 2.12.4 source: `src/path/desktop.rs`, `app_data_dir` implementation. The replacement resolver is still Proposed until contract-tested.
- [eframe documentation](https://docs.rs/eframe/latest/eframe/) for renderer, font, clipboard, accessibility, and optional inspection features; recheck the selected locked release during implementation.
- [egui accessibility guidance](https://github.com/emilk/egui/blob/main/docs/accessibility.md) for AccessKit labels and testing.
- [rfd 0.16.0 documentation](https://docs.rs/rfd/0.16.0/rfd/) for native dialogs and platform constraints.

## Review state

Self-review: the staged scope, crate ownership, temporary legacy path, data-format compatibility, restore transaction, application draft revisions, and delivery gates have been checked for consistency. Dependency versions, native font/IME behavior, Windows resolver parity, legacy installer metadata, and native package execution are implementation validation gates, not verified outcomes of this design.

The user approved the written specification on 2026-10-20. Next: create the detailed implementation plan under `docs/superpowers/plans/`. Implementation starts only after the plan review.
