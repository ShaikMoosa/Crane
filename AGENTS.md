# Crane development guidance

## Product and stack

Read `PRODUCT.md` before product decisions and `DESIGN.md` before interface changes. Crane is a local-first Markdown notebook. The user has selected the `saas` skill as the governing stack for future development: pure Rust, egui/eframe for the native interface, and wgpu for rendering.

The current released implementation is still Rust + Tauri v2 with a static `ui/` frontend. Treat those files and their tests as the migration baseline; the native migration design and implementation plan must be reviewed before replacing them. Use `tauri-vanillajs-desktop` only to inspect or maintain the legacy implementation where necessary, not to choose the target architecture.

New native application code must not introduce Tauri, Electron, a webview, JavaScript/TypeScript UI code, a Node.js runtime/build dependency, remote scripts, remote fonts, or required services. Preserve the offline workflow; accounts, billing, sync, collaboration, and agent automation require separate product scope.

## Superpowers workflow

This project uses the installed official Superpowers development methodology. Invoke its applicable Codex skills by their qualified names instead of paraphrasing them:

1. `superpowers:using-superpowers` at the start of work to route the request through applicable skills.
2. `superpowers:brainstorming` before a new feature or behavior; record approved project designs under `docs/superpowers/specs/`.
3. `superpowers:writing-plans` after design approval; store plans under `docs/superpowers/plans/`.
4. `superpowers:test-driven-development` during implementation: write and observe the failing behavior test before production code.
5. `superpowers:systematic-debugging` for every unexpected result; establish a reproducible root cause before changing code.
6. `superpowers:requesting-code-review` before delivery when review support is available.
7. `superpowers:verification-before-completion` before any success claim; run fresh full checks and report blockers exactly.

Do not require subagents or worktrees when the current harness, user request, or environment does not authorize them. Apply the inline `superpowers:executing-plans` route instead.

## Engineering rules

- During migration, keep existing DOM code in the legacy `ui/` directory. Put the new native interface in an egui/eframe desktop shell; keep domain/application contracts independent of UI, operating-system, network, identity-provider, and billing-provider dependencies.
- Expose typed domain operations, never arbitrary filesystem or shell primitives. Validate commands, imported content, selected paths, sizes, and destructive confirmations at their boundaries.
- Persist through a temporary sibling file followed by rename. A failed save must be visible to the UI.
- Keep the persisted schema versioned and tolerate future unknown fields where practical.
- Preserve existing note IDs, bodies, timestamps, states, tags, and version-1 JSON/backup compatibility. Validate migration and rollback with isolated fixtures before using a real library.
- Keep modules focused; test domain transformations and application transactions without a GUI.
- Add no dependency without a demonstrated need. Pin releases through `Cargo.lock` once generated.
- Do not copy Bear or Opensidian source, text, logos, themes, screenshots, or assets.

## Verification

While the legacy app remains, run JavaScript tests with `node --test tests/*.test.mjs`. From `src-tauri/`, run `cargo fmt -- --check`, `cargo test --locked`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, and `cargo build --locked`. Exercise the legacy frontend against its browser preview and Tauri IPC bridges when changing it.

Once the native Cargo workspace exists, also run `cargo fmt --all -- --check`, `cargo test --workspace --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo build --workspace --all-targets --all-features --locked`. Verify dependency boundaries, legacy-data compatibility, failed-save recovery, and native editor/dialog behavior. Validate installers on Windows; containers do not prove native GUI or installer behavior.

