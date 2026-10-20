# Crane native migration compatibility evidence

This document records synthetic-data compatibility evidence for the staged native migration. It never uses note bodies from the installed Crane library.

## Version-1 schema matrix

| Fixture or condition | Expected behavior | Automated evidence |
| --- | --- | --- |
| `all-states.json` | Preserve active, pinned, archived, and trashed notes, order, Unicode, tags, flags, IDs, and timestamps | `crane-domain` `supported_v1_fixtures_round_trip_semantically` |
| `empty.json` | Load as an intentionally empty library; never seed onboarding notes | `crane-domain` semantic round trip and `crane-storage` `empty_library_is_not_reseeded` |
| `extended.json` | Preserve unknown store and note fields without interpreting them | `crane-domain` `unknown_store_and_note_fields_survive_round_trip` and storage repository round trip |
| `future-schema.json` | Reject visibly without writing or seeding | `crane-domain` `future_schema_is_rejected_without_reinterpretation` |
| Malformed JSON | Reject visibly and preserve exact existing bytes | `crane-storage` `corruption_or_permission_error_never_seeds` |
| Inaccessible/non-file path | Reject visibly and do not create seed data | `crane-storage` `corruption_or_permission_error_never_seeds` |

## Windows data-root contract

On 2026-10-20, the platform resolver and a Tauri mock application configured with the production identifier `app.crane.desktop` returned the same location:

`C:\Users\Admin\AppData\Roaming\app.crane.desktop`

The automated legacy regression is `main_tests::shared_resolver_matches_tauri_app_data_dir`. The native resolver is also checked independently by `legacy_data_dir_uses_the_roaming_root_and_identifier`. Neither test opens `notes.json`.

## Write-safety contract

Storage writes reserve an exclusive temporary sibling, flush it, and delegate same-directory replacement to the platform adapter. Existing reparse-point leaves or ancestors are refused. Replacement failures preserve the destination and remove only the operation-owned temporary file; unrelated siblings remain. A process-held exclusive library guard rejects a second writer.

The version-1 JSON repository bounds a library read at 256 MiB. This keeps an untrusted library from allocating without limit and aligns the readable store with Crane's maximum backup envelope class. An oversized library fails visibly; it is never replaced by seed data.

## Transfer and restore transaction evidence

Task 5 moved import, export, backup, inspection, cancellation, and restore orchestration into typed application commands. `failed_import_adds_zero_notes` verifies that invalid UTF-8, unsupported extensions, title/body limits, and the 64 MiB aggregate limit leave memory and disk unchanged. `restore_requires_fresh_unrevoked_confirmation` verifies explicit cancellation, superseding inspections, intervening library revisions, one-use tokens, and the 300-second monotonic expiry.

Every accepted restore writes a validated, size-limited safety backup before attempting repository replacement. `safety_failure_skips_replacement` and `replacement_failure_keeps_memory_and_disk` verify the ordering and failure states. Backup serialization uses a bounded writer, so output beyond 256 MiB stops before a destination is published; `backup_size_is_checked_before_any_target_is_published` exercises the same path with an injected small limit.

Library export reserves its final name, writes through an exclusive hidden temporary directory, and publishes only by final rename. `windows_compatible_export_names_create_distinct_files` exercises `Plan/plan`, `Σ/ς`, `COM¹`, and `LPT³` on Windows. `failed_library_export_keeps_unrelated_siblings_and_hides_final_folder` verifies that failure removes only operation-owned artifacts. `normalized_existing_backup_target_requires_confirmation` verifies overwrite confirmation after `.crane-backup.json` normalization.

The retained Tauri shell now delegates to the same in-process `Library` and `FilesystemTransfers` used by the native migration boundary. `legacy_adapter_transfer_path_uses_shared_application` imports fixture Markdown through that shared command path. JavaScript bridge coverage verifies the narrow `cancel_restore` IPC used to revoke a dismissed confirmation; no generic filesystem IPC was added.

On 2026-10-20, the Task 5 debug binary was launched under the unique process path `target/crane-task5.exe`. The Windows `Import Markdown` picker opened with the `.md` filter, Escape closed it without selecting data, the picker process window disappeared, and Crane's Data control returned enabled. The temporary copied executable was then removed. This is native-dialog cancellation evidence; installer-level repetition remains part of the release gate.
