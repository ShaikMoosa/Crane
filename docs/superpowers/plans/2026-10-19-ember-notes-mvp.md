# Ember Notes MVP implementation plan

## File map

- `src-tauri/src/model.rs`: note/store types, validation, tag extraction, state transitions, unit tests.
- `src-tauri/src/persistence.rs`: app-data path, load, atomic save, persistence tests.
- `src-tauri/src/main.rs`: Tauri state, narrow commands, setup, handler registration.
- `ui/core.mjs`: pure sorting, filtering, excerpts, dates, and Markdown preview helpers.
- `ui/bridge.mjs`: Tauri IPC bridge plus browser-preview bridge.
- `ui/app.js`: DOM state, autosave coordination, keyboard actions, responsive navigation.
- `ui/index.html`: accessible three-pane application shell.
- `ui/styles.css`: responsive manuscript-inspired visual system.
- `tests/core.test.mjs`: frontend behavior tests.

## Tasks

1. Write failing frontend tests for tag normalization, note visibility, ordering, excerpts, and safe Markdown rendering; run them and confirm expected failures.
2. Implement only the pure helpers required for those tests; run the full JavaScript suite.
3. Write Rust model tests for seed data, tag extraction, validation, and recoverable state transitions.
4. Implement Rust model types and behavior, then persistence tests and atomic storage.
5. Wire Tauri commands and configuration with a restrictive CSP and minimal capabilities.
6. Build the semantic HTML shell and responsive layout from the approved visual direction.
7. Connect the UI to the bridge, implement autosave/error states, and add keyboard shortcuts.
8. Run JavaScript verification, JSON/config checks, static accessibility checks, and Rust checks where toolchain availability permits.
9. Inspect desktop and compact-width renders in one batch, fix material issues once, and record remaining limitations.

