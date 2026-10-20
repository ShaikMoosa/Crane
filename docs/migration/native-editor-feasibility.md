# Native editor feasibility evidence

This document records evidence for the transitional native Crane shell. It does
not claim complete writing-interface parity; the Task 7 Windows acceptance pass
still owns keyboard, accessibility, DPI, clipboard, IME, selection, undo, long
note, and spellcheck decisions.

## 2026-10-20 Task 6 check

- Built `crane-desktop` with eframe/egui 0.37.0 and wgpu 30.0.1 on the pinned
  Rust 1.99.1 toolchain. The resolved Windows feature tree contains AccessKit,
  clipboard, default/system fonts, DX12, and WGSL. It does not contain glow,
  inspection, eframe persistence, Vulkan, Metal, GLES, or web screen-reader
  features.
- Launched `cargo run -p crane-desktop --locked -- --preview-data-dir
  artifacts/test-data/native-editor`. Initialization reported `NVIDIA GeForce
  RTX 3050 Laptop GPU (Dx12, DiscreteGpu)` and remained running until the test
  process was stopped.
- The preview root was created explicitly, marked with
  `.crane-preview-fixture`, and seeded a schema-version-1 `notes.json` containing
  three synthetic onboarding notes. Startup without `--preview-data-dir` failed
  visibly and did not fall back to the production library.
- Automated draft tests cover the 550 ms debounce, save coalescing, a delayed
  acknowledgement arriving after newer typing, switching only after the latest
  draft is durable, retained content and visible retry after failure, and close
  refusal until a retry succeeds.
- Automated worker tests cover capacity-eight request/result channels, immediate
  queue-full feedback, ordered retention of all accepted work, one mutation at a
  time, wake-on-result, and a synthetic note saved through the worker and reopened
  from an isolated JSON library.

The available computer-control surface did not enumerate the native eframe
window, so pointer/keyboard automation could not independently exercise typing,
paste, undo, or IME in this pass. Those checks remain explicitly open rather than
being inferred from a successful build or from egui feature flags. Native
spellcheck has likewise not been established and must not be claimed as parity.

## 2026-10-20 Task 7 automated writing-interface check

- Native kittest coverage now exercises named controls for create, search, nested tags, state changes, preview/theme toggles, title focus, Tab traversal, and explicit sub-680 editor/list/library navigation.
- The editor binds egui directly to the active draft's persistent title and body strings. Automated Unicode Markdown save-and-switch coverage confirms the text survives the worker transaction without a per-frame field replacement.
- Query and Markdown presentation caches are revision keyed and have regression tests proving unrelated frames and theme changes do not recompute them.
- Version-1 native settings load and save atomically; tests cover missing defaults, future-schema rejection, and a failed replacement retaining the previous file.
- The Task 7 binary launched against `artifacts/test-data/task7-native-writing` and reported the NVIDIA GeForce RTX 3050 Laptop GPU through DX12. The computer-control inventory still returned no native applications and exposed no Windows-listing function, so the preview was stopped without recording unobserved interactions as passes.
- The physical Windows matrix and the open spellcheck product decision are maintained in `windows-acceptance.md`. They remain milestone gates rather than implied passes.
