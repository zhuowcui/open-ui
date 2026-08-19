# Continuous Integration

Open UI uses hosted GitHub Actions as a portable pre-merge gate and the pinned
Chromium 147 workstation pipeline as the authoritative pixel-parity gate. Both
are required; they answer different questions.

## Hosted checks

The workflows under `.github/workflows/` run on Ubuntu 24.04 with read-only
repository permissions.

| Check | Contract |
|---|---|
| `native-build (Debug)` | Generate the portable GN graph, build `hello_world`, and run it. |
| `native-build (Release)` | Repeat the portable native smoke test with release flags. |
| `rust-parity` | Check Rust formatting and run the style, text, layout, and paint test suites. |
| `python-accountability` | Run the SP13-R through SP16 closure/porter tests, verify the immutable SP13-R ledgers, and require the accountability audit to pass 7/7. |
| `clang-format` | Require every tracked C/C++ source under `src/`, `include/`, and `examples/` to match clang-format 18. |
| `gn-format` | Require every tracked `.gn` and `.gni` file to pass `gn format --dry-run`. |

The native smoke workflow installs Ubuntu's `generate-ninja` package directly.
Do not replace it with a shallow `depot_tools` clone: the wrapper requires a
bootstrapped Chromium checkout and was the cause of the original build and GN
format failures.

The Rust job overrides the machine-specific paths in
`bindings/rust/.cargo/config.toml` with the hosted compiler and builds Skia from
source. Its cache is keyed by the Rust toolchain and Cargo manifests. The
checked-in Cargo config remains the source of the pinned local raster policy.

## Local equivalents

Native smoke build, using any current standalone `gn`, `ninja`, and Clang:

```bash
gn gen out/CI-Debug --args='is_debug=true'
ninja -C out/CI-Debug hello_world
./out/CI-Debug/hello_world

gn gen out/CI-Release --args='is_debug=false'
ninja -C out/CI-Release hello_world
./out/CI-Release/hello_world
```

Rust and accountability gates in the pinned development environment:

```bash
cd bindings/rust
cargo fmt --all --check
cargo test --locked \
  --package openui-style \
  --package openui-text \
  --package openui-layout \
  --package openui-paint
cd ../..

python3 -m unittest \
  tools.wpt.test_sp13r_multicol_closure \
  tools.wpt.test_sp14_text_port \
  tools.wpt.test_sp15_closure \
  tools.wpt.test_sp16_closure
python3 tools/wpt/generate_sp13r_multicol_closure.py --check
python3 tools/accountability/audit.py
```

Native formatting:

```bash
find src include examples -type f \
  \( -name '*.cc' -o -name '*.h' -o -name '*.c' \) -print0 | \
  sort -z | xargs -0 --no-run-if-empty clang-format-18 --dry-run --Werror

git ls-files -z '*.gn' '*.gni' | \
  xargs -0 --no-run-if-empty -n1 gn format --dry-run
```

## Pixel-parity gate

Hosted CI does not claim Chromium pixel parity. Exact comparisons require the
repository-pinned Linux Chromium 147 binary, fonts, FreeType behavior, and the
800x600 runner profile. Before closing any parity sprint, run the exact-ID
target manifest and then the complete WPT suite without resume, regenerate the
authoritative artifacts twice, and require byte-identical output plus audit
7/7. The canonical commands remain in `docs/progress/current-status.md`.

Focused runs overwrite `summary.json`. Snapshot and restore the authoritative
full summary unless the focused run is intentionally replacing it.

## Pull-request procedure

1. Run the local equivalents appropriate to the change.
2. Push the branch and wait for every required Actions check to complete.
3. Inspect failures with `gh pr checks <number>` and the linked job logs.
4. Mark the PR ready only after the hosted checks and the applicable pinned
   parity evidence are green.
5. Merge using the repository's squash policy and verify the resulting `main`
   commit and post-merge Actions run.
