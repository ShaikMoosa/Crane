# Crane import, export, and backup design

## Intent

Give a single offline Crane user direct ownership of their notes. The feature must make notes portable as ordinary Markdown and make the complete Crane library recoverable without exposing general filesystem access to the frontend or weakening the existing atomic persistence guarantees.

Success means the user can import Markdown, export one note or the usable library, create a complete backup, and safely replace the current library from a valid backup. No operation requires an account, network connection, Node.js runtime, or remote service.

## Scope

This increment includes:

- importing one or more UTF-8 Markdown files;
- exporting the selected note as Markdown;
- exporting active and archived notes into a new folder tree;
- creating a versioned full-library Crane backup;
- restoring a full-library backup after validation, confirmation, and an automatic safety backup;
- native file and folder dialogs, progress state, cancellation, success feedback, and actionable failures.

This increment does not include attachments, media rewriting, ZIP archives, PDF or document export, cloud destinations, scheduled backups, encryption, conflict merging, import from proprietary Bear archives, or changing Crane's primary storage from JSON to Markdown files.

## Chosen architecture

Privileged transfer work belongs to a new Rust `transfer` module beside `model` and `persistence`. The frontend calls narrow domain commands and never receives a general filesystem capability or supplies an arbitrary path.

The command surface is:

- `import_markdown()`
- `export_note(id)`
- `export_library()`
- `create_backup()`
- `inspect_backup()`
- `restore_backup(confirmation_token)`

The Rust commands open native file or folder dialogs. Adding the official Tauri dialog plugin is justified because user-selected native paths are required and the current dependency set has no dialog facility. File parsing, validation, filename generation, directory layout, and writes remain in Rust.

`inspect_backup` validates the selected file and returns a short-lived, in-memory confirmation token plus backup metadata. `restore_backup` accepts only that token; it does not accept a path. The token is invalidated after use, cancellation, a new inspection, or application exit. This preserves the rule that IPC callers cannot request arbitrary filesystem access.

Pure transfer functions accept paths internally so they can be tested with temporary directories without invoking dialogs. Dialog-facing commands remain thin wrappers around those functions.

## Store ownership and concurrency

Crane's versioned JSON store remains the authoritative internal database. Markdown is a portable interchange format, not live storage. This preserves IDs, timestamps, pinned, archived, and trashed state without introducing directory watchers, metadata sidecars, or filename-based identity.

Every transfer command first flushes a pending editor save through the existing frontend flow. Rust reads or changes the store under the existing mutex. File parsing and export preparation should happen outside the mutex where possible; the mutex is held only while taking a consistent snapshot or committing a validated replacement/import.

Import adds every validated note and persists the store once. Restore persists one validated replacement. A failure before persistence leaves the in-memory and on-disk library unchanged. A persistence failure is reported and must not be presented as success.

## Markdown export format

Markdown exports are UTF-8 `.md` files with this shape:

```markdown
# Note title

Existing Markdown body
```

The separate Crane title becomes the first level-one heading. The body follows unchanged apart from the single separating blank line. A blank title exports as `# Untitled note`.

Single-note export opens a Save dialog with a sanitized title-based filename. It works for an active, archived, or trashed selected note because it is an explicit note-level action.

Library export opens a folder picker, then creates a new timestamped `Crane Export YYYY-MM-DD HHmmss` folder containing:

- `Notes/` for notes that are neither archived nor trashed;
- `Archive/` for archived notes that are not trashed.

Trashed notes are excluded from library Markdown exports. Pinned state and timestamps are not encoded in Markdown in this version.

Windows-reserved characters and trailing spaces or periods are removed from filenames. Empty or reserved device names fall back to `Untitled note`. Name collisions receive deterministic numeric suffixes such as `Meeting notes 2.md`. The sanitized stem is limited to 120 Unicode characters before adding a collision suffix and `.md`.

The complete export is written to a temporary sibling folder. Crane renames that folder to the final timestamped name only after every file has been flushed successfully. If writing fails, Crane reports the error and removes only the temporary folder it created; it never modifies an existing user folder.

## Markdown import behavior

Import accepts one or more `.md` files selected through the native picker. Every file must be valid UTF-8 and fit Crane's existing 2 MiB note-body limit after parsing. The cumulative selected input is capped at 64 MiB to avoid unbounded memory use. Files with another extension are rejected.

For each file:

1. The first non-empty line is treated as the title only when it is a single level-one Markdown heading (`# Title`).
2. That heading and one immediately following blank line are removed from the body.
3. If no qualifying heading exists, the filename stem becomes the title and the body remains unchanged.
4. The title and body must pass the existing note validation limits.
5. Tags are derived from the body with Crane's existing tag extractor.
6. Crane assigns a new ID and current created/updated timestamps.
7. The note starts active, unpinned, unarchived, and not trashed.

Imports always create notes; they never overwrite or deduplicate existing notes. The entire selection is parsed and validated before the store changes. If any file fails, no notes are imported and the error identifies the failing filename and reason.

## Backup format

Full backups use the `.crane-backup.json` suffix and a versioned envelope:

```json
{
  "format": "crane-backup",
  "backupVersion": 1,
  "createdAt": 1760870400000,
  "appVersion": "0.1.0",
  "store": {
    "schemaVersion": 1,
    "notes": []
  }
}
```

The backup contains every note, including archived and trashed notes, and preserves all store metadata. Unknown envelope fields are ignored for forward compatibility, but an unknown `backupVersion` or store `schemaVersion` is rejected. Restore validates the envelope, every note, unique non-empty IDs, and the current schema before offering confirmation. Backup files larger than 256 MiB are rejected before parsing.

Creating a backup opens a Save dialog with a timestamped default filename and writes through a temporary sibling followed by an atomic replacement. Cancelling the dialog is a successful no-op.

## Restore transaction

Restore deliberately replaces rather than merges the current library.

1. The user selects a `.crane-backup.json` file.
2. Rust reads, parses, and fully validates it without modifying current state.
3. The frontend receives the backup date and counts for total, active, archived, and trashed notes plus a confirmation token.
4. The frontend explains that the current library will be replaced and that Crane will create a safety backup first.
5. After confirmation, Rust verifies and consumes the token.
6. Rust writes the current store as a versioned backup under the application-data `backups/` directory using a unique `pre-restore-<timestamp>.crane-backup.json` name.
7. Only after the safety backup is flushed does Rust atomically persist the validated replacement to `notes.json`.
8. Rust swaps the in-memory store and returns its complete contents to refresh the interface.

If validation, safety-backup creation, or replacement persistence fails, Crane keeps the current in-memory and on-disk library. The error states which phase failed. A cancelled inspection or confirmation changes nothing.

## User interface

A compact `Data` menu sits at the bottom of the charcoal library rail. It contains Import Markdown, Export Library, Create Backup, and Restore Backup. The existing note action menu gains Export Note.

Controls follow Crane's current navigation styling, visible focus treatment, and 40 px desktop/44 px compact target rules. The Data menu collapses consistently with the library rail at medium widths and remains labelled in the compact one-pane layout. It does not introduce a dashboard, card surface, or new accent color.

Transfer controls show an in-progress state and reject duplicate activation. Only controls that could conflict with the active transfer are disabled. Existing notes remain readable; operations that change the store require a successful pending-save flush first.

Restore confirmation is a semantic modal dialog with focus containment, Escape cancellation, an explicit destructive action, and focus restoration to the invoking control. It displays the backup timestamp and note counts. The destructive confirmation uses the existing danger color; ember remains reserved for ordinary active state and successful evidence.

Success is announced through the existing toast/status pattern with concrete copy such as `12 notes imported` or `Library exported`. Cancellation is silent. Errors remain visible long enough to act on and use the filename and failure phase where safe.

## Frontend bridge behavior

`ui/bridge.mjs` exposes the same high-level transfer methods in native and browser-preview modes. The native branch invokes the Rust commands. Browser preview uses injected test files and in-memory/download adapters so the interface can exercise success, cancellation, and failure without claiming native filesystem access. Preview-only behavior must be clearly isolated and must not alter the native security boundary.

The app updates its note state from the complete store returned by successful import or restore. Export and backup commands return structured outcomes with `completed` or `cancelled` status and relevant counts; cancellation is not represented as an exception.

## Error behavior

- Unsupported extension or non-UTF-8 Markdown identifies the rejected filename.
- An oversized Markdown file or batch reports the applicable limit.
- A malformed backup distinguishes unreadable JSON, wrong format, unsupported version, and invalid note data.
- A failed import commits no notes.
- A failed library export does not expose a partial final folder.
- A failed restore leaves the current library selected and unchanged.
- A save that cannot be flushed prevents a transfer that depends on current note state.
- Unexpected filesystem details are logged or wrapped, while user-facing messages remain concise and do not expose sensitive path information unnecessarily.

## Testing strategy

Rust unit tests cover:

- Markdown serialization and parsing, including blank titles and heading detection;
- tag extraction after import;
- title/body size validation;
- safe filenames, Windows reserved names, truncation, and collision suffixes;
- active/archive export placement and trash exclusion;
- full backup round trips including archived and trashed notes;
- envelope and store-version rejection;
- malformed, oversized, and duplicate-ID backup rejection;
- all-or-nothing batch import;
- safety-backup creation before replacement;
- preservation of the current store when any restore phase fails;
- cancellation outcomes and one-use restore tokens.

Rust integration-style tests call path-accepting transfer functions with isolated test directories. Dialog wrappers remain thin and receive targeted command tests.

JavaScript tests cover:

- bridge dispatch through browser-preview and Tauri IPC branches;
- pending-save flush before store-changing operations;
- success, cancellation, and error UI state;
- transfer control locking;
- restore metadata rendering and explicit confirmation;
- state refresh after import and restore;
- accessible menu and dialog behavior where it can be tested without a browser DOM harness.

Manual Windows verification covers native open/save/folder dialogs, keyboard-only operation, export/import round trips, archive placement, trash exclusion, backup replacement, automatic safety-backup creation, cancellation, failure messaging, and an NSIS release build.

## Acceptance criteria

- The application performs all transfer operations offline.
- The frontend cannot request reads or writes for an arbitrary path.
- Exported Markdown is human-readable and can be imported back into Crane.
- Library Markdown export contains active and archived notes but no trashed notes.
- A full backup preserves all notes and metadata, including trash.
- Import is all-or-nothing and never overwrites an existing note.
- Restore cannot modify the library until the selected backup is fully validated and the user confirms replacement.
- Restore creates and flushes a safety backup before replacing the current store.
- Any failed import or restore leaves the current library unchanged.
- The interface remains keyboard accessible, responsive, and consistent with Crane's design system.
- Automated JavaScript and Rust checks pass, and the Windows installer still builds.

