# Crane native Windows acceptance

This is the physical-device gate for the native writing interface. Automated tests establish data and widget behavior, but they do not substitute for Windows input-method, assistive-technology, monitor-scaling, or installer checks.

## Automated evidence — 2026-10-20

| Area | Evidence | Status |
| --- | --- | --- |
| Layout breakpoints | `navigation::create_note_focuses_title_and_layout_breakpoints_are_exact`; `navigation::sub_680_layout_has_explicit_editor_list_and_library_navigation` | Pass |
| Native workflow | `navigation::native_navigation_searches_filters_and_changes_note_state` | Pass |
| Persistent editor data | `editor::markdown_edit_survives_save_and_note_switch`; draft coordinator tests | Pass |
| Failure retention | `editor::failed_or_missing_save_keeps_draft_and_selection`; draft retry/close tests | Pass |
| Accessibility names and Tab order | `accessibility.rs` | Pass |
| Query and preview caching | navigation/editor cache tests | Pass |
| Settings safety | `crane-storage --test settings` | Pass |
| Safe outbound links | `crane-platform::links::only_http_schemes_are_accepted` | Pass |
| Transfer sequencing and mutation lock | `apps/desktop/tests/transfers.rs` | Pass |
| Restore confirmation, failure recovery, and focus return | `apps/desktop/tests/restore_dialog.rs` | Pass |
| Data disclosure, dialog identity, focus containment, and 44-point transfer targets | `apps/desktop/tests/accessibility.rs`; `apps/desktop/tests/restore_dialog.rs` | Pass |
| Normalized note/backup overwrite confirmation contract | `crates/storage/tests/transfers.rs` | Pass |

The headless egui harness intentionally permits tofu for glyphs absent from its small embedded test font set. The test still verifies that Unicode Markdown is retained and persisted exactly. The production binary enables eframe's local system-font provider plus licensed egui fallbacks and does not fetch fonts from the network.

## Physical Windows checks still required

Run these against an isolated preview data directory. Record Windows version, display scale, keyboard layout, and result. Never point the preview at the real Crane library.

- [ ] At 100%, 150%, and 200% display scaling: inspect all three panes, focus indicators, clipping, wrapping, and the 720-by-560 minimum window.
- [ ] Enter Latin, Devanagari, Arabic/RTL, CJK, combining marks, and emoji through both keyboard and an IME; save, switch notes, restart, and verify exact content.
- [ ] Copy and paste plain and multi-line text between Crane and another native Windows application in both directions.
- [ ] Exercise mouse and keyboard selection, Home/End, word movement, Ctrl+A/C/X/V/Z/Y, and multi-step undo/redo without switching notes unexpectedly.
- [ ] Exercise one very long unbroken line and a synthetic note close to the 2 MiB body limit; verify scrolling, visible validation, and no UI hang.
- [ ] Complete create/edit/search/tag/state/preview/theme/retry flows using keyboard only.
- [ ] With Narrator, verify names, roles, values, focus order, selection state, save failure, Retry, and single-pane navigation announcements.
- [ ] Disconnect networking and verify theme changes, local font fallbacks, Markdown preview, persistence, and restart remain functional.
- [ ] Verify reduced motion removes the one-shot save feedback transition once that visual transition is enabled.

## Physical transfer checks still required

Use only these disposable locations when this checklist is executed:

- Preview library: `artifacts/test-data/native-transfers/library`
- Markdown inputs: `artifacts/test-data/native-transfers/import`
- Export destination: `artifacts/test-data/native-transfers/export`
- Backup destination: `artifacts/test-data/native-transfers/backups`

These locations are reserved for the future physical run; they were not used or marked as evidence by the headless UI and adapter tests.

- [ ] Exercise the native multi-file Markdown picker, including quiet cancellation and one invalid file aborting the batch.
- [ ] Export an active, archived, and explicitly selected trashed note through the native save dialog; verify `.md` normalization and overwrite behavior.
- [ ] Export the library through the native folder picker; verify active notes under `Notes/`, archived notes under `Archive/`, and trash exclusion.
- [ ] Create a complete backup through the native save dialog, including normalized overwrite confirmation and quiet cancellation.
- [ ] Inspect and cancel restore, then inspect and confirm restore; verify all counts, the pre-restore safety backup, and the replacement library after restart.
- [ ] Inject safety-backup and replacement-write failures against the disposable library and verify the previous library remains readable.

## Known parity decision

Native spellcheck is not established in the egui editor. On 2026-10-20 the user approved deferring native spellcheck to a separately designed future feature so the staged migration can continue. Crane does not claim spellcheck parity in this migration.

The Codex computer-control surface used for this implementation does not enumerate the native eframe window, so these physical checks remain open. A successful build, AccessKit test, or wgpu launch is not recorded as a manual pass.
