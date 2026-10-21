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

## Installer upgrade/rollback cycle — 2026-10-21

Run on this Windows host (per-user scope, silent `/S`, current user; no administrator rights). The real library at `%APPDATA%\app.crane.desktop\notes.json` was never opened by either app: its SHA-256 was `64FDA31B409537A9E5EB8F35882AE6244EA524B3A460DCB142599D3A86859BC7` before and after the cycle (1,916 bytes, unchanged).

| Step | Evidence | Status |
| --- | --- | --- |
| Legacy installer retained | `artifacts/legacy/Crane_0.1.0_x64-setup.exe` (2,113,680 bytes, SHA-256 `D4192FAFFEE613B9B33C7CE0D602114720258BAF903F2F4630FE1D2576956A39`), built with tauri-cli 2.12.3 + NSIS 3.10; smoke-verified | Pass |
| Legacy install | Uninstall key `Crane`, DisplayVersion 0.1.0, program in `%LOCALAPPDATA%\Crane` (payload exe SHA-256 `9F6C21D1363FD5C6D6120DF426A3B49C0FEFD8BB5E87E63F3F46CC1578CF5A47`) | Pass |
| Upgrade (first attempt) | Native install left **both** registrations (`Crane` 0.1.0 and `app.crane.desktop` 0.2.0) and both programs | Fail — fixed |
| Installer fix | `packaging/windows/crane.nsi` now runs the legacy uninstaller silently during install and removes leftover legacy key/program files; data untouched | Fixed |
| Upgrade (after fix) | Exactly one registration: `app.crane.desktop` 0.2.0 in `%LOCALAPPDATA%\Programs\Crane`; legacy dir gone; installed exe SHA-256 `32948212EAF307B80D70D828D32B8572EA31235BB3187662CFEAC8047FC35384` matches the staged build | Pass |
| Installed app launch/reopen | Installed `crane.exe --preview-data-dir artifacts/test-data/task10-acceptance`: created `.crane-preview-fixture` + `notes.json` (schemaVersion 1, 3 notes); second launch left `notes.json` byte-identical (SHA-256 `51ABFCD9B1701D96E71AFA67CCC226FC43010DEC5B606F82A338216DAEE265A1`); both launches closed cleanly | Pass |
| Native uninstall | Registration and `%LOCALAPPDATA%\Programs\Crane` removed; data preserved | Pass |
| Rollback | Legacy 0.1.0 reinstalled (registration + program restored), then uninstalled; machine returned to no-Crane-install state | Pass |
| Named suites | `crane-storage --test cutover` 5/5, `crane-platform --test instance` 3/3, `crane-desktop --test startup` 5/5 | Pass |
| Native installer | `artifacts/windows/Crane_0.2.0_x64-setup.exe` (SHA-256 `562EDEC70E6348DDABB25D56CABC3E13A6F61D347B798C7383D99A5E722D4055` after the takeover fix) + `build-provenance.txt` | Pass |

Still open (not claimed by this cycle): legacy GUI launch against an isolated profile (launching it here would open the real library; v1 data compatibility is covered by fixture/adapter/cutover tests), interactive picker/dialog/IME/Narrator/GPU checks below, and the legacy MSI bundle (WiX step failed: no `.ico` icon tracked; the NSIS bundle above is the retained legacy installer). Signing: both installers unsigned; no credentials supplied.

## Known parity decision

Native spellcheck is not established in the egui editor. On 2026-10-20 the user approved deferring native spellcheck to a separately designed future feature so the staged migration can continue. Crane does not claim spellcheck parity in this migration.

The Codex computer-control surface used for this implementation does not enumerate the native eframe window, so these physical checks remain open. A successful build, AccessKit test, or wgpu launch is not recorded as a manual pass.
