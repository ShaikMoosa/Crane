# Crane development guidance

## Product and stack

Read `PRODUCT.md` before product decisions and `DESIGN.md` before interface changes. Crane is a local-first Markdown notebook. The user has selected the `saas` skill as the governing stack for future development: pure Rust, egui/eframe for the native interface, and wgpu for rendering.

The native Rust implementation (0.2.0) is the released implementation. The legacy Rust + Tauri v2 shell with its static `ui/` frontend was retired in migration Task 10 after the Windows upgrade/rollback acceptance cycle; the retained 0.1.0 NSIS installer lives outside the repo build directories (`artifacts/legacy/`, with SHA-256) for rollback. The dated 2026-10-19 Tauri specs/plans are historical and superseded by the 2026-10-20 native migration design. `tauri-vanillajs-desktop` is historical context only; never reintroduce its stack.

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

- The native interface lives in the egui/eframe desktop shell (`apps/desktop`); keep domain/application contracts independent of UI, operating-system, network, identity-provider, and billing-provider dependencies. Enforce this with `scripts/check-layers.ps1` and `scripts/check-native-release.ps1`.
- Expose typed domain operations, never arbitrary filesystem or shell primitives. Validate commands, imported content, selected paths, sizes, and destructive confirmations at their boundaries.
- Persist through a temporary sibling file followed by rename. A failed save must be visible to the UI.
- Keep the persisted schema versioned and tolerate future unknown fields where practical.
- Preserve existing note IDs, bodies, timestamps, states, tags, and version-1 JSON/backup compatibility. Validate migration and rollback with isolated fixtures before using a real library.
- Keep modules focused; test domain transformations and application transactions without a GUI.
- Add no dependency without a demonstrated need. Pin releases through `Cargo.lock` once generated.
- Do not copy Bear or Opensidian source, text, logos, themes, screenshots, or assets.

## Verification

The legacy JavaScript tests and Tauri crate were removed with the retired shell; behavior coverage lives in the Rust suites mapped by `docs/migration/behavior-parity.md`. Browser-preview data migration is documented in `docs/migration/browser-preview-export.md`.

Run `cargo fmt --all -- --check`, `cargo test --workspace --all-features --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo build --workspace --all-targets --all-features --locked`. Also run `powershell -NoProfile -File scripts/check-layers.ps1`, `powershell -NoProfile -File scripts/check-native-release.ps1`, and the script `-SelfTest` modes. Verify dependency boundaries, legacy-data compatibility, failed-save recovery, and native editor/dialog behavior. Validate installers on Windows; containers do not prove native GUI or installer behavior.

