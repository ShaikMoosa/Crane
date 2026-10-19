# Crane development guidance

## Product and stack

Read `PRODUCT.md` before product decisions and `DESIGN.md` before interface changes. This is a local-first Rust + Tauri v2 application with a static `ui/` frontend. Runtime code must not require Node.js, remote scripts, fonts, or services.

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

- Keep DOM and interaction concerns in `ui/`; keep persistence and privileged operations in Rust.
- Treat IPC arguments as untrusted. Expose domain operations, never arbitrary filesystem or shell primitives.
- Persist through a temporary sibling file followed by rename. A failed save must be visible to the UI.
- Keep the persisted schema versioned and tolerate future unknown fields where practical.
- Keep frontend modules focused and test domain transformations without the DOM.
- Add no dependency without a demonstrated need. Pin releases through `Cargo.lock` once generated.
- Do not copy Bear or Opensidian source, text, logos, themes, screenshots, or assets.

## Verification

Run JavaScript tests with `node --test tests/*.test.mjs`. From `src-tauri/`, run `cargo fmt -- --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build` when the Rust toolchain is available. Exercise the frontend against both its browser preview bridge and the Tauri IPC bridge.

