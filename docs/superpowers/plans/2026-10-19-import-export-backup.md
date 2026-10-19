# Crane Import, Export, and Backup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe offline Markdown import/export and complete Crane backup/restore with native dialogs and no general frontend filesystem access.

**Architecture:** A new Rust `transfer` subsystem owns Markdown conversion, filename rules, backup validation, and transactional file writes. Thin Tauri commands open native dialogs and operate on the existing mutex-protected JSON store; a focused JavaScript controller coordinates pending saves, structured outcomes, and the new Data menu and restore confirmation dialog.

**Tech Stack:** Rust 2021, Tauri v2, `tauri-plugin-dialog`, Serde/JSON, vanilla HTML/CSS/JavaScript, Node's built-in test runner.

**Spec:** `docs/superpowers/specs/2026-10-19-import-export-backup-design.md`

## Global Constraints

- Runtime behavior is fully offline and must not require Node.js, remote scripts, fonts, services, or accounts.
- The frontend calls domain commands only; it never supplies an arbitrary filesystem path or receives a general filesystem capability.
- Crane's versioned JSON store remains authoritative; Markdown is an interchange format.
- Import accepts UTF-8 `.md` files up to the existing 2 MiB body limit and caps a selected batch at 64 MiB.
- Backups use `.crane-backup.json`, include every note and metadata field, and are rejected above 256 MiB.
- Sanitized Markdown filename stems are limited to 120 Unicode characters before suffix and extension.
- Library Markdown export includes active and archived notes, excludes trash, and exposes no partial final folder.
- Import and restore are all-or-nothing; restore creates and flushes a safety backup before replacing the current store.
- UI work follows `DESIGN.md`: charcoal rail, compact controls, existing ember/danger roles, 40 px desktop targets, 44 px compact targets, visible focus, and reduced-motion support.
- Do not add attachments, ZIP/PDF/document export, sync, scheduled backups, encryption, conflict merging, or proprietary Bear import.

## Review Focus

- Windows filenames that differ only by case, use reserved device names, or end in spaces/dots must still export to unique valid files; Task 1 pins this with manifest tests.
- A Markdown batch containing one invalid UTF-8, oversized, or invalid-title file must add zero notes; Task 1 pins this with an all-or-nothing parsing test.
- Malformed backups, duplicate note IDs, unsupported versions, and oversized input must be rejected before confirmation; Task 2 pins each class with validation tests.
- A safety-backup or replacement-write failure must leave the current in-memory store unchanged, and a stale/reused restore token must fail; Tasks 2 and 3 pin these transaction boundaries.
- Pending unsaved edits, user cancellation, and repeated clicks must not start conflicting transfers or appear as errors; Tasks 4 and 5 pin controller and UI state.

---

## File map

- Create `src-tauri/src/transfer/mod.rs`: shared transfer outcomes, summaries, limits, errors, and module exports.
- Create `src-tauri/src/transfer/markdown.rs`: Markdown parsing/serialization, safe filenames, batch preparation, export manifests, and transactional directory export.
- Create `src-tauri/src/transfer/backup.rs`: backup envelope, validation, inspection metadata, pending restore data, atomic backup writes, and restore transaction.
- Modify `src-tauri/src/persistence.rs`: expose reusable atomic byte writing while preserving current store-save behavior.
- Modify `src-tauri/src/model.rs`: add store-wide validation used by backup restore.
- Modify `src-tauri/src/main.rs`: register dialog plugin, extend app state, implement narrow transfer commands, and register handlers.
- Modify `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`: add and lock the official Tauri dialog plugin.
- Keep `src-tauri/capabilities/default.json` narrow: no JavaScript filesystem or dialog permission is added.
- Create `ui/transfer-controller.mjs`: pending-save gating, structured outcome handling, restore inspection/confirmation flow, and duplicate-operation locking.
- Modify `ui/bridge.mjs`: native IPC and browser-preview transfer methods.
- Modify `ui/app.js`: bind transfer controller to application state and DOM feedback.
- Modify `ui/index.html`: Data menu, Export Note action, icons, and semantic restore dialog.
- Modify `ui/styles.css`: Data menu, busy state, and accessible confirmation dialog styles.
- Create `tests/transfer-bridge.test.mjs`: preview/native dispatch and structured outcome tests.
- Create `tests/transfer-controller.test.mjs`: save gating, cancellation, refresh, error, and duplicate-operation tests.
- Create `tests/data-ui.test.mjs`: static semantic/accessibility checks for the new controls.
- Modify `PRODUCT.md` and `README.md`: record the shipped offline transfer capabilities and usage.

### Task 1: Markdown transfer core

**Files:**
- Create: `src-tauri/src/transfer/mod.rs`
- Create: `src-tauri/src/transfer/markdown.rs`
- Modify: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/main.rs` (module declaration only)

**Interfaces:**
- Consumes: `model::{extract_tags, validate_note, Note, Store, MAX_BODY_BYTES}`.
- Produces: `parse_markdown(name: &str, bytes: &[u8], timestamp: u64, id: String) -> Result<Note, TransferError>`, `serialize_markdown(note: &Note) -> String`, `prepare_import(files: Vec<NamedBytes>, timestamp: u64, ids: impl Iterator<Item = String>) -> Result<Vec<Note>, TransferError>`, `build_export_manifest(store: &Store) -> Result<ExportManifest, TransferError>`, `write_note_export(path: &Path, note: &Note) -> Result<(), TransferError>`, and `write_library_export(destination: &Path, manifest: &ExportManifest, folder_name: &str) -> Result<ExportSummary, TransferError>`.
- Produces model helper: `validate_store(store: &Store) -> Result<(), String>` checks schema, every note, and unique non-empty IDs.

- [ ] **Step 1: Write failing Markdown and filename tests in `src-tauri/src/transfer/markdown.rs`**

  Add `mod transfer;` in `main.rs`, declare `pub mod markdown;` in `transfer/mod.rs`, then add tests named `round_trips_title_and_body`, `uses_filename_when_h1_is_absent`, `rejects_non_utf8_and_oversized_inputs`, `rejects_the_entire_batch_when_one_file_fails`, `builds_notes_and_archive_manifests_without_trash`, and `sanitizes_reserved_case_insensitive_filename_collisions`. Assert that export/import preserves title/body/tags, invalid batches yield no prepared notes, `CON.md` falls back safely, `Plan.md` and `plan.md` receive distinct names, stems are at most 120 characters, archive placement is correct, and trash is absent.

- [ ] **Step 2: Run the focused tests and confirm RED**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml transfer::markdown -- --nocapture`

  Expected: FAIL because the transfer module and functions do not exist.

- [ ] **Step 3: Implement the pure Markdown conversion and manifest interfaces**

  Create the types `NamedBytes { name: String, bytes: Vec<u8> }`, `ExportEntry { relative_path: PathBuf, contents: Vec<u8> }`, `ExportManifest { entries: Vec<ExportEntry> }`, `ExportSummary { exported_count: usize, folder_name: String }`, and a displayable `TransferError`. Enforce `.md`, UTF-8, per-note, 64 MiB batch, title, ID, tag, trash, archive, reserved-name, case-insensitive collision, and 120-character requirements exactly as specified.

- [ ] **Step 4: Add store-wide validation tests and implementation in `model.rs`**

  Tests: `validate_store_rejects_wrong_schema` and `validate_store_rejects_duplicate_ids`. Implement `validate_store` using `SCHEMA_VERSION`, `validate_note`, and a `HashSet` of IDs.

- [ ] **Step 5: Implement transactional library directory writing**

  Write all manifest entries beneath a uniquely named temporary sibling directory, flush each file, rename only after all writes succeed, and clean up only the temporary directory on error. Add `does_not_publish_partial_export_when_a_write_fails` using an internal failing-writer seam rather than platform permission tricks.

- [ ] **Step 6: Run Rust tests and formatting**

  Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml transfer::markdown -- --nocapture`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml model::tests -- --nocapture`

  Expected: all focused tests PASS.

- [ ] **Step 7: Commit Task 1**

  ```bash
  git add src-tauri/src/transfer src-tauri/src/model.rs src-tauri/src/main.rs
  git commit -m "feat: add Markdown transfer core"
  ```

### Task 2: Versioned backup and restore transaction

**Files:**
- Create: `src-tauri/src/transfer/backup.rs`
- Modify: `src-tauri/src/transfer/mod.rs`
- Modify: `src-tauri/src/persistence.rs`

**Interfaces:**
- Consumes: `model::{validate_store, Store, SCHEMA_VERSION}` and `persistence::write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String>`.
- Produces: `BackupEnvelope { format, backup_version, created_at, app_version, store }`, `BackupInspection { created_at, total, active, archived, trashed }`, `PendingRestore { token, inspection, replacement }`, `serialize_backup(store: &Store, created_at: u64, app_version: &str) -> Result<Vec<u8>, TransferError>`, `inspect_backup(bytes: &[u8], token: String) -> Result<PendingRestore, TransferError>`, and `restore_transaction(current: &Store, replacement: Store, write_safety: impl FnOnce(&Store) -> Result<(), TransferError>, write_replacement: impl FnOnce(&Store) -> Result<(), TransferError>) -> Result<Store, TransferError>`.

- [ ] **Step 1: Write failing backup-envelope tests**

  Declare `pub mod backup;` in `transfer/mod.rs`, then add tests named `backup_round_trip_preserves_all_note_states`, `rejects_wrong_format_and_future_versions`, `rejects_duplicate_ids_and_invalid_notes`, and `rejects_backup_over_256_mib_before_json_parse`. Assert exact `format = "crane-backup"`, `backupVersion = 1`, state preservation including trash, and distinct error variants for JSON, format, backup version, store version, validation, and size.

- [ ] **Step 2: Run focused backup tests and confirm RED**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml transfer::backup -- --nocapture`

  Expected: FAIL because `transfer::backup` is not implemented.

- [ ] **Step 3: Extract reusable atomic byte writing**

  Implement `pub(crate) fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String>` in `persistence.rs`. Make `save` serialize and delegate to it. Preserve temporary sibling creation, `sync_all`, closed handle, and platform-specific atomic replacement behavior. Extend the existing persistence test to prove replacement still works.

- [ ] **Step 4: Implement the backup envelope, validation, and summaries**

  Use camelCase Serde fields and ignore unknown fields by default. Validate the byte-size limit before parsing, then format/version/store/note/unique-ID rules. Count active as neither archived nor trashed, archived as archived and not trashed, and trashed independently.

- [ ] **Step 5: Write failing restore-order tests**

  Add `writes_safety_backup_before_replacement`, `safety_failure_skips_replacement`, and `replacement_failure_returns_error_without_new_in_memory_store`. Use closure spies to record phase order and inject deterministic failures.

- [ ] **Step 6: Implement `restore_transaction` and atomic backup file writing**

  Return the replacement store only after both closures succeed. Add `write_backup(path, envelope)` using `write_atomic_bytes`; generate `pre-restore-<timestamp>.crane-backup.json` under the caller-provided application-data backup directory.

- [ ] **Step 7: Run focused and full Rust tests**

  Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml transfer::backup -- --nocapture`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml persistence::tests -- --nocapture`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked`

  Expected: all tests PASS.

- [ ] **Step 8: Commit Task 2**

  ```bash
  git add src-tauri/src/transfer src-tauri/src/persistence.rs
  git commit -m "feat: add versioned Crane backups"
  ```

### Task 3: Native dialogs and narrow Tauri commands

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/main.rs`
- Verify unchanged: `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: Task 1 Markdown functions, Task 2 backup functions, existing `AppState.path`, store mutex, `now_millis`, and `new_id`.
- Produces serialized IPC types: `TransferOutcome<T> { status: TransferStatus, data: Option<T> }`, `TransferStatus::{Completed, Cancelled}` serialized in camelCase, `ImportSummary { store: Store, imported_count: usize }`, `BackupInspectionPayload { token: String, created_at: u64, total: usize, active: usize, archived: usize, trashed: usize }`, and the six spec commands. `AppState.store` becomes `Arc<Mutex<Store>>`, and `AppState` gains `pending_restore: Arc<Mutex<Option<PendingRestore>>>`, so blocking file tasks can own cloned state handles safely.

- [ ] **Step 1: Write failing outcome/token tests in `main.rs`**

  Add tests named `cancelled_outcome_serializes_without_data`, `restore_token_is_single_use`, and `stale_restore_token_is_rejected`. Factor token consumption into `take_pending_restore(pending: &Mutex<Option<PendingRestore>>, token: &str) -> Result<PendingRestore, String>` so it is deterministic without a dialog.

- [ ] **Step 2: Run command-state tests and confirm RED**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml main_tests -- --nocapture`

  Expected: FAIL because transfer outcome and pending-restore state are absent.

- [ ] **Step 3: Add and register `tauri-plugin-dialog`**

  Add the Tauri v2-compatible dependency, register `.plugin(tauri_plugin_dialog::init())`, and use the Rust `DialogExt` API. Do not add `dialog:*` or filesystem permissions to `capabilities/default.json` because JavaScript never invokes those plugin APIs directly.

- [ ] **Step 4: Implement the Markdown import/export commands**

  Implement `import_markdown`, `export_note`, and `export_library`. Each opens its own native picker, maps cancellation to `TransferStatus::Cancelled`, uses only picker-returned paths internally, and returns concise structured data without full filesystem paths. Declare file-heavy commands `async` and move blocking reads/writes onto `tauri::async_runtime::spawn_blocking`; hold the store mutex only while taking a snapshot or committing imported notes.

- [ ] **Step 5: Implement the backup inspection and restore commands**

  Implement `create_backup`, `inspect_backup`, and `restore_backup` with the same async/blocking-I/O boundary. `inspect_backup` replaces any older pending restore; `restore_backup` consumes the token before attempting the transaction and holds the store mutex across the safety-backup/replacement commit so note edits cannot interleave.

- [ ] **Step 6: Register all handlers and initialize pending state**

  Add commands to `generate_handler!`; initialize `pending_restore` in setup. Keep existing note commands unchanged except for shared helper use needed by imports.

- [ ] **Step 7: Verify Rust, dependency lock, and capability scope**

  Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

  Run: `cargo test --manifest-path src-tauri/Cargo.toml --locked`

  Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --all-features -- -D warnings`

  Run: `cargo build --manifest-path src-tauri/Cargo.toml --locked`

  Expected: all commands exit 0; `capabilities/default.json` still contains only `core:default`.

- [ ] **Step 8: Commit Task 3**

  ```bash
  git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/main.rs src-tauri/capabilities/default.json
  git commit -m "feat: expose native data transfer commands"
  ```

### Task 4: Frontend bridge and transfer controller

**Files:**
- Create: `ui/transfer-controller.mjs`
- Modify: `ui/bridge.mjs`
- Create: `tests/transfer-bridge.test.mjs`
- Create: `tests/transfer-controller.test.mjs`

**Interfaces:**
- Consumes: native commands from Task 3 and callbacks `flushSave() -> Promise<boolean>`, `selectedNoteId() -> string | null`, `replaceStore(store)`, and `notify(message, kind)`.
- Produces bridge methods `importMarkdown()`, `exportNote(id)`, `exportLibrary()`, `createBackup()`, `inspectBackup()`, and `restoreBackup(token)` plus `createTransferController(dependencies)` with methods of the same user actions and read-only `busy` state.

- [ ] **Step 1: Write failing bridge dispatch tests**

  In `tests/transfer-bridge.test.mjs`, test native invocation names/arguments, preview-adapter delegation, cancellation as `{ status: "cancelled", data: null }`, and preview adapter failures. Reset globals and use cache-busting dynamic import URLs so native and preview branches are isolated.

- [ ] **Step 2: Run bridge tests and confirm RED**

  Run: `node --test tests/transfer-bridge.test.mjs`

  Expected: FAIL because the six bridge methods are absent.

- [ ] **Step 3: Implement bridge methods and preview adapter boundary**

  Native methods call `window.__TAURI__.core.invoke` with only a note ID or confirmation token when required. Preview methods delegate to `globalThis.__CRANE_PREVIEW_TRANSFER__`; when no preview adapter is installed, return a cancelled outcome rather than pretending a filesystem write occurred.

- [ ] **Step 4: Write failing transfer-controller tests**

  Cover `flushes_before_import_and_restore`, `aborts_when_pending_save_fails`, `cancellation_is_silent`, `completed_import_replaces_store_and_reports_count`, `completed_restore_replaces_store`, `errors_unlock_controls`, and `duplicate_activation_while_busy_is_ignored`. Assert callback order and exact state effects.

- [ ] **Step 5: Implement `createTransferController`**

  Centralize one-operation-at-a-time locking, pending-save gating, structured outcome handling, and state replacement. Keep DOM access out of this module; `app.js` supplies callbacks.

- [ ] **Step 6: Run all JavaScript tests**

  Run: `node --test tests/*.test.mjs`

  Expected: all tests PASS.

- [ ] **Step 7: Commit Task 4**

  ```bash
  git add ui/bridge.mjs ui/transfer-controller.mjs tests/transfer-bridge.test.mjs tests/transfer-controller.test.mjs
  git commit -m "feat: add frontend transfer controller"
  ```

### Task 5: Accessible Data menu and restore confirmation UI

**Files:**
- Modify: `ui/index.html`
- Modify: `ui/styles.css`
- Modify: `ui/app.js`
- Create: `tests/data-ui.test.mjs`

**Interfaces:**
- Consumes: Task 4 controller and `BackupInspectionPayload`.
- Produces DOM controls `#data-menu-button`, `#data-menu`, `#import-markdown`, `#export-library`, `#create-backup`, `#inspect-backup`, `#export-note`, and semantic `#restore-dialog` with confirm/cancel controls and metadata fields.

- [ ] **Step 1: Write failing static UI tests**

  Read `ui/index.html` and `ui/styles.css` in `tests/data-ui.test.mjs`. Assert the Data menu is inside `.library-footer`, every action is a real button, the trigger has `aria-expanded` and `aria-controls`, restore uses `<dialog>` with labelled title/description, the confirm button uses the danger class, and compact rules provide at least 44 px targets.

- [ ] **Step 2: Run the UI test and confirm RED**

  Run: `node --test tests/data-ui.test.mjs`

  Expected: FAIL because the Data menu and restore dialog are absent.

- [ ] **Step 3: Add semantic markup and Crane-consistent styles**

  Add only the icons required for import/export/backup. Reuse the action-menu visual vocabulary, keep the menu attached to the rail footer, preserve medium collapsed-rail behavior, restore full labels in compact one-pane mode, add visible focus and disabled/busy styles, and suppress nonessential transitions under reduced motion.

- [ ] **Step 4: Wire the controller in `app.js`**

  Instantiate the controller with existing save/state/toast callbacks. Bind all five visible actions, close menus consistently, update `state.notes` and selection after import/restore, render backup metadata, call `showModal()`, restore focus on close, support Escape/cancel, and disable conflicting transfer controls while busy.

- [ ] **Step 5: Add UI-state regression tests to the controller suite**

  Add assertions that inspection alone never restores, cancel never calls `restoreBackup`, confirm passes exactly the inspection token, an expired-token error leaves existing state untouched, and repeated activation remains locked until the original promise settles.

- [ ] **Step 6: Run all frontend tests and syntax checks**

  Run: `node --check ui/app.js`

  Run: `node --check ui/bridge.mjs`

  Run: `node --check ui/transfer-controller.mjs`

  Run: `node --test tests/*.test.mjs`

  Expected: all commands exit 0 and all tests PASS.

- [ ] **Step 7: Commit Task 5**

  ```bash
  git add ui/index.html ui/styles.css ui/app.js tests/data-ui.test.mjs tests/transfer-controller.test.mjs
  git commit -m "feat: add data transfer interface"
  ```

### Task 6: Documentation, end-to-end verification, and installer

**Files:**
- Modify: `PRODUCT.md`
- Modify: `README.md`
- Verify: all implementation files from Tasks 1-5

**Interfaces:**
- Consumes: complete native and frontend flows.
- Produces: documented user workflow and a verified Windows NSIS installer; no generated `target/` artifacts are committed.

- [ ] **Step 1: Update product and user documentation**

  Move Markdown import/export and complete backup/restore from MVP exclusions into capabilities. Document active/archive/trash export behavior, replacement restore with automatic safety backup, supported extensions, offline operation, and where the Data menu lives.

- [ ] **Step 2: Run the complete automated verification suite**

  From the repository root run: `node --test tests/*.test.mjs`

  From `src-tauri/` run: `cargo fmt -- --check`

  From `src-tauri/` run: `cargo test --locked`

  From `src-tauri/` run: `cargo clippy --locked --all-targets --all-features -- -D warnings`

  From `src-tauri/` run: `cargo build --locked`

  Expected: zero JavaScript or Rust test failures, formatting differences, Clippy diagnostics, or build failures. Sandbox-only incremental hard-link fallback messages do not count as product diagnostics.

- [ ] **Step 3: Manually exercise the native flows on Windows**

  From `src-tauri/` run: `cargo tauri dev`

  Verify: single-note export/import round trip; active/archive folder placement; trash exclusion; complete backup; restore confirmation counts; replacement restore; safety backup creation; dialog cancellation; keyboard navigation; compact layout; and clear invalid-file errors. Record any issue as a failing automated test before fixing it.

- [ ] **Step 4: Build the installable package**

  From `src-tauri/` run in PowerShell: `$env:CARGO_BUILD_JOBS = '1'; cargo tauri build --bundles nsis`

  Expected: exit 0 and `target/release/bundle/nsis/Crane_0.1.0_x64-setup.exe` exists. Confirm the installer remains unsigned unless a code-signing certificate is explicitly supplied.

- [ ] **Step 5: Verify the final diff and repository state**

  Run: `git diff --check`

  Run: `git status --short`

  Confirm only intended source, test, documentation, and lockfile changes are present; `target/` and exported user data remain untracked/ignored.

- [ ] **Step 6: Commit Task 6**

  ```bash
  git add PRODUCT.md README.md
  git commit -m "docs: explain Crane data portability"
  ```

- [ ] **Step 7: Request final code review and address findings**

  Use `superpowers:requesting-code-review` for the complete branch diff. Fix every Critical or Important issue through a failing test first, rerun the full verification suite, and report any deliberately deferred Minor issue.

