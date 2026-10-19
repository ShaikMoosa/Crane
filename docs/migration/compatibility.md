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
