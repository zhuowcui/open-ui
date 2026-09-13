# Contributing to Open UI

Open UI v0.2 is developed in the pure-Rust workspace under `bindings/rust`.
The root GN/C++ Blink backend and the SP2 experiments are historical evidence;
do not add new product features to them.

## Getting started

1. Fork and clone the repository.
2. Install the prerequisites in [the development guide](docs/DEVELOPMENT.md).
3. Create a focused branch.
4. Build and test the locked Rust workspace.

```bash
cd bindings/rust
cargo build --workspace --locked
cargo test --workspace --locked
```

Headless builds are the default. Use the `linux` feature when changing the
native X11/Wayland runtime. A Chromium checkout is not required for supported
builds or release artifacts.

## Code and API rules

- Format Rust with `cargo fmt`; run Clippy for changed packages.
- Keep every public crate and internal dependency on the coordinated v0.2
  version declared by the workspace.
- Treat `openui-style` as the only public source of style value definitions.
- Do not add runtime property-name or CSS-value string setters.
- Keep window-system dependencies behind Cargo features so headless consumers
  do not acquire Linux display dependencies.
- Document the safety contract for every unsafe block and FFI entry point, and
  add negative tests for malformed inputs.
- Preserve single-thread affinity for mutable engine state. Only immutable
  scene snapshots may cross to the compositor thread.

The generated C header is C11-compatible. Public symbols use the `oui_`
prefix, opaque types use the `Oui` prefix, and exported functions return
`OuiStatus`. New public structs require `struct_size` and `abi_version`; UTF-8
inputs are length-delimited.

## Generated files

Never hand-edit generated style, ABI, migration, or closure artifacts. Change
their schema/source and rerun the corresponding generator. Verification must
be read-only and deterministic.

```bash
python3 tools/style/generate_properties.py --check
python3 tools/ffi/generate_ffi.py --check
python3 tools/release/generate_v02_contract.py --check
python3 tools/wpt/generate_sp20_closure.py --check
```

## Correctness and release gates

Behavior changes need proportionate Rust, C ABI, conformance, and compile-fail
coverage. Rendering changes must pass the frozen 5,731-case exact replay with
zero tolerance. Do not introduce test-ID branches, reference substitution,
synthetic geometry, hidden fallbacks, network access, or unclassified
exclusions.

At minimum, run the relevant package tests and:

```bash
python3 tools/release/build_v02_linux.py --verify-source
python3 tools/ffi/verify_abi.py
python3 tools/accountability/audit.py
git diff --check
```

See [the CI contract](docs/CI.md) for the hosted matrix, scheduled hardening,
and release proof. Performance and accessibility regressions in required
scenarios are release blockers.

## Pull requests and commits

- Keep each change logically scoped and include tests for behavior changes.
- Update API, migration, and unsupported-feature documentation together.
- Record significant design decisions in `docs/adr/` using the template.
- Use a concise component-oriented subject such as
  `engine: invalidate intrinsic sizes after text mutation`.
- Do not include build outputs or release credentials.

Issue reports should include the OS and architecture, Rust/compiler versions,
selected backend, a minimal reproduction, and screenshots or traces for visual
and interaction failures.
