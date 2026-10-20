# Crane migration behavior parity

This matrix records the behavior protected by the released Tauri/DOM implementation while Crane moves to a pure-Rust egui/eframe desktop application. It is a migration checklist, not permission to copy the legacy UI structure. Domain behavior is moved to Rust tests; DOM and IPC assertions are replaced by native widget, command, dialog, and platform acceptance checks.

## Native acceptance items

- **NA-01 — Native library identity:** the installed native app resolves and opens the existing `app.crane.desktop` roaming data directory without creating a second library.
- **NA-02 — Native data controls:** import, export, backup, and restore are reachable through native controls with accessible labels and at least 44-pixel interaction targets.
- **NA-03 — Busy-state integrity:** native transfer operations disable conflicting note mutations and always unlock after cancellation or failure.
- **NA-04 — Restore confirmation:** inspection never mutates data; a labelled native confirmation dialog passes the inspected one-use token, cannot be dismissed after commit begins, and preserves the current library on stale-token or write failure.
- **NA-05 — Native file dialogs:** cancellation is quiet; path selection and overwrite confirmation use native dialogs; errors stay visible and actionable.
- **NA-06 — Transfer command boundary:** the desktop shell dispatches typed application commands with narrow arguments and no arbitrary filesystem or shell primitive.
- **NA-07 — Atomic export/backup:** native adapters publish only complete exports/backups, reject unsafe or duplicate relative destinations, and remove only their own temporary artifacts on failure.
- **NA-08 — Native Markdown view:** headings, paragraphs, quotes, lists, read-only tasks, fenced code, nested emphasis, safe links, literal HTML, and image alt text render from the safe Rust presentation tree without executing or fetching content.
- **NA-09 — Native writing flow:** create, select, edit, save, search, nested-tag filter, pin, archive, trash, restore, preview, and theme controls are exposed through named native widgets. A note switch waits for the dirty draft to save, and a failed save retains both draft and selection with a visible Retry action.
- **NA-10 — Native responsive navigation:** the 900/680 point breakpoints map to wide, collapsed-rail, and single-pane layouts. The sub-680 harness has explicit editor-to-list and list-to-library navigation while the production window retains its 720-point minimum.
- **NA-11 — Native settings:** theme and reduced-motion settings use a versioned JSON document written through a temporary sibling and atomic replacement. Missing settings use defaults; future schemas fail visibly; a failed replacement preserves the old document.
- **NA-12 — Native accessibility:** primary buttons and text fields have stable AccessKit names, keyboard Tab traversal reaches search and the editor, and theme/state changes do not depend on pointer-only gestures.
- **NA-13 — Cache discipline:** query results recompute only when the store revision or filter changes; Markdown preview recomputes only when the selected draft revision changes. Theme changes do not invalidate either cache.

## JavaScript test parity

| Legacy test | Rust coverage or native replacement |
| --- | --- |
| `branding: browser preview persists Crane data under the Crane namespace` | `platform::legacy_data_dir_uses_the_roaming_root_and_identifier`, `src-tauri::shared_resolver_matches_tauri_app_data_dir`, then **NA-01** |
| `core: extractTags normalizes and deduplicates nested tags` | `domain::extracts_unique_nested_tags` and `domain::extracts_hyphenated_nested_and_unicode_tags_with_legacy_grammar` |
| `core: filterNotes combines collection, tag, and text query` | `queries::search_combines_title_body_and_tags` |
| `core: filterNotes keeps trash separate from active notes` | `queries::collections_keep_active_archive_and_trash_separate` |
| `core: sortNotes keeps pinned notes first then orders by recency` | `queries::active_notes_sort_pinned_first_then_by_recency` and `queries::equal_sort_keys_keep_original_store_order` |
| `core: excerptFrom removes markdown and tags without returning an empty excerpt` | `markdown::excerpt_removes_markup_code_images_and_tags` |
| `core: renderMarkdown escapes HTML and rejects unsafe link protocols` | `preview::html_is_inert_and_unsafe_links_are_plain_text`, then **NA-08** |
| `core: renderMarkdown supports lists, tasks, emphasis, and safe links` | `preview::safe_links_are_typed_but_images_keep_alt_text_only`, `preview::parses_nested_formatting_code_quotes_lists_and_read_only_tasks`, then **NA-08** |
| `data-ui: Data menu lives in the library footer and uses real buttons` | Replace DOM assertion with **NA-02** |
| `data-ui: Data trigger exposes expanded state and controlled menu` | Replace ARIA/DOM assertion with native disclosure-state coverage in **NA-02** |
| `data-ui: restore confirmation is a labelled semantic dialog` | Replace DOM assertion with **NA-04** |
| `data-ui: compact transfer controls preserve 44 pixel targets` | Replace CSS assertion with **NA-02** |
| `data-ui: busy transfers block note mutations until their store snapshot completes` | Replace JavaScript state assertion with **NA-03** |
| `data-ui: restore replacement cannot be dismissed after commit starts` | Legacy assertion retained; shared lifecycle coverage is `restore_requires_fresh_unrevoked_confirmation`, then native widget behavior remains **NA-04** |
| `data-ui: closing restore confirmation revokes its one-use token` | Legacy bridge assertion plus `restore_requires_fresh_unrevoked_confirmation`; native dialog behavior remains **NA-04** |
| `transfer-controller: flushes_before_import_and_restore` | `application::failed_create_save_and_state_changes_preserve_committed_store`, plus native transfer sequencing in **NA-03** and **NA-04** |
| `transfer-controller: aborts_when_pending_save_fails` | `application::failed_create_save_and_state_changes_preserve_committed_store`, then **NA-03** |
| `transfer-controller: cancellation_is_silent` | Replace browser cancellation assertion with **NA-05** |
| `transfer-controller: completed_import_replaces_store_and_reports_count` | `failed_import_adds_zero_notes`, `legacy_adapter_transfer_path_uses_shared_application`, and domain import coverage; native presentation remains **NA-03** |
| `transfer-controller: completed_restore_replaces_store` | `backup::backup_builder_counts_every_note_state`, `safety_failure_skips_replacement`, and `replacement_failure_keeps_memory_and_disk`; native presentation remains **NA-04** |
| `transfer-controller: errors_unlock_controls` | Replace JavaScript state assertion with **NA-03** |
| `transfer-controller: native string errors stay visible to the user` | Replace bridge-specific assertion with **NA-05** |
| `transfer-controller: duplicate_activation_while_busy_is_ignored` | Replace JavaScript state assertion with **NA-03** |
| `transfer-controller: inspection_alone_never_restores` | **NA-04** |
| `transfer-controller: cancel_never_calls_restoreBackup` | `restore_requires_fresh_unrevoked_confirmation`, plus native widget behavior in **NA-04** |
| `transfer-controller: confirm passes exactly the inspected token` | `restore_requires_fresh_unrevoked_confirmation`, plus native widget behavior in **NA-04** |
| `transfer-controller: expired token error leaves existing state untouched` | `restore_requires_fresh_unrevoked_confirmation` and `replacement_failure_keeps_memory_and_disk` |
| `transfer-bridge: native transfer methods dispatch narrow command arguments` | Replace Tauri IPC assertion with **NA-06** |
| `transfer-bridge: preview transfer methods delegate to the injected adapter` | Browser-preview adapter is retired; native dialog delegation is covered by **NA-05** and **NA-06** |
| `transfer-bridge: missing preview adapter returns a normalized cancelled outcome` | Browser-only behavior is retired; native cancellation is covered by **NA-05** |
| `transfer-bridge: preview adapter failures are not hidden` | Browser-only adapter is retired; native failures are covered by **NA-05** |

## Legacy Rust test parity

| Legacy Rust test | Rust coverage or native replacement |
| --- | --- |
| `model::extracts_unique_nested_tags` | `domain::extracts_unique_nested_tags` |
| `model::validates_note_size_limits` | `domain::title_and_body_limits_are_exact_utf8_byte_limits` |
| `model::validate_store_rejects_wrong_schema` | `domain::future_schema_is_rejected_without_reinterpretation` |
| `model::validate_store_rejects_duplicate_ids` | `domain::blank_and_duplicate_ids_are_rejected` |
| `model::validate_store_rejects_timestamps_javascript_cannot_render` | `domain::timestamps_respect_the_renderable_boundary` |
| `main::legacy_adapter_creates_saves_and_reopens_an_isolated_library` | Retained during migration; application coverage is `create_commits_once_and_publishes_after_success`, `save_updates_owned_fields_and_preserves_identity_extensions_and_flags`, and storage coverage is `repository_round_trip_preserves_supported_extensions` |
| `main::shared_resolver_matches_tauri_app_data_dir` | `platform::legacy_data_dir_uses_the_roaming_root_and_identifier`, then **NA-01** |
| `main::cancelled_outcome_serializes_without_data` | Tauri response shape is retired; native cancellation remains **NA-05** |
| `main::restore_token_is_single_use` | `restore_requires_fresh_unrevoked_confirmation` |
| `main::stale_restore_token_is_rejected` | `restore_requires_fresh_unrevoked_confirmation` |
| `main::formats_export_timestamp_in_utc` | Shared storage adapter formatting exercised by export tests; native picker presentation remains **NA-07** |
| `main::bounded_reader_rejects_before_retaining_bytes_past_the_limit` | `storage::bounded_reader_stops_at_limit_plus_one` |
| `main::normalized_backup_targets_that_exist_require_confirmation` | `normalized_existing_backup_target_requires_confirmation`; native dialog acceptance remains **NA-05** |
| `backup::backup_round_trip_preserves_all_note_states` | `backups::backup_builder_counts_every_note_state` and `backups::version_one_backup_round_trip_preserves_all_unknown_fields` |
| `backup::rejects_wrong_format_and_future_versions` | `backups::malformed_future_and_oversized_backups_are_rejected` |
| `backup::rejects_duplicate_ids_and_invalid_notes` | Domain validation plus `inspection_reports_domain_counts`; inspection delegates through `inspect_backup` before confirmation state is created |
| `backup::rejects_backup_over_256_mib_before_json_parse` | `backups::malformed_future_and_oversized_backups_are_rejected` |
| `backup::refuses_to_create_a_backup_larger_than_the_restore_limit` | `backup_size_is_checked_before_any_target_is_published` through the bounded serializer |
| `backup::rejects_an_unrenderable_backup_envelope_timestamp` | `backups::invalid_backup_timestamp_is_rejected` |
| `backup::writes_safety_backup_before_replacement` | `safety_failure_skips_replacement` and `replacement_failure_keeps_memory_and_disk` |
| `backup::safety_failure_skips_replacement` | `safety_failure_skips_replacement` |
| `backup::replacement_failure_returns_error_without_new_in_memory_store` | `replacement_failure_keeps_memory_and_disk` |
| `markdown::round_trips_title_and_body` | `markdown::imports_heading_as_title_and_preserves_body` |
| `markdown::uses_filename_when_h1_is_absent` | `markdown::import_uses_filename_when_h1_is_absent` |
| `markdown::rejects_non_utf8_and_oversized_inputs` | `markdown::rejects_non_utf8_and_oversized_batches` |
| `markdown::rejects_the_entire_batch_when_one_file_fails` | `markdown::invalid_file_aborts_the_entire_batch` |
| `markdown::builds_notes_and_archive_manifests_without_trash` | `markdown::export_manifest_separates_active_and_archive_and_excludes_trash` |
| `markdown::sanitizes_reserved_case_insensitive_filename_collisions` | `markdown::export_names_are_windows_safe_and_collision_resistant` |
| `markdown::duplicate_manifest_paths_abort_instead_of_overwriting_an_export` | Exclusive `create_new` entries in `write_library_export_with`; native acceptance remains **NA-07** |
| `markdown::rejects_a_batch_larger_than_the_limit` | `markdown::rejects_non_utf8_and_oversized_batches` |
| `markdown::does_not_publish_partial_export_when_a_write_fails` | `failed_library_export_keeps_unrelated_siblings_and_hides_final_folder` |

## Deliberate compatibility decisions

- **Trash filtering correction:** the legacy JavaScript returned every trashed note before applying the selected tag or query. Native `query_note_ids` applies tag and text filters to Trash just like the other collections. `queries::trash_applies_tag_and_query_filters_instead_of_returning_early` locks this intended user behavior.
- **Saved tag grammar is unchanged:** tag extraction remains whitespace-led `#tag` parsing with Unicode letters/numbers plus `_`, `-`, and non-empty `/`-separated segments. Existing stored tags and version-1 fixtures are not reinterpreted.
- **Unsafe Markdown stays data:** raw HTML is literal text, only `http://` and `https://` destinations become links, unsafe schemes in literal HTML are neutralized, and images retain alt text without a fetchable URL.

## Native Task 7 evidence

- `apps/desktop/tests/navigation.rs` covers creation focus, search, exact nested-tag filtering, pin/archive/trash/restore flows, cache invalidation, all three layout breakpoints, and explicit single-pane navigation.
- `apps/desktop/tests/editor.rs` covers Unicode Markdown persistence through save and note switching, safe preview/theme behavior, preview cache stability, and retained draft/selection on failure.
- `apps/desktop/tests/accessibility.rs` queries every primary interactive control by role and name and traverses the native Tab order into search and the note body.
- `crates/storage/tests/settings.rs` covers missing defaults, round trips, future-schema rejection, and synthetic atomic replacement failure without changing the previous settings file.
- The remaining physical Windows checks and the spellcheck parity decision are tracked in `docs/migration/windows-acceptance.md`; they are not inferred from automated coverage.
