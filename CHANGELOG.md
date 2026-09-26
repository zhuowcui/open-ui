# Changelog

## 0.2.0 — release candidate

- Converged the Rust framework, headless renderer, C ABI, and Linux runtime on
  the retained pure-Rust Open UI engine.
- Added a generated typed style schema and compile-time checked `view!` style
  literals; removed supported HTML, CSS-text, and generic string-style paths.
- Added immutable scenes, deterministic lifecycle invalidation, hit testing,
  core controls/editing, typed animation timelines, resources, and retained
  accessibility semantics.
- Added X11 and Wayland operation, OpenGL presentation, software fallback,
  Linux clipboard/IME/event normalization, and AccessKit integration for Rust
  applications.
- Replaced the historical C API with a versioned, panic-contained 84-symbol
  typed ABI and matching headless C examples.
- Added 36 application conformance cases, exact 5,731-render replay, fuzzing,
  sanitizers, Miri, performance/memory gates, and reproducible SDK/package
  tooling.

Final release is blocked on the open qualifications in
`docs/v02/release.md`, including C-owned Linux window parity, retained promoted
animation layers, physical-GPU/AT-SPI runs, and signed dual-architecture
artifacts.
