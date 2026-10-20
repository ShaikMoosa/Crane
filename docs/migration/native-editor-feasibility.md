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
