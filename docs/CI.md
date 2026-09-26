# Continuous integration and release gates

The supported CI surface is the pure-Rust v0.2 workspace. The workflows have
read-only repository permissions unless a tag job explicitly needs an
attestation identity.

## Pull-request gates

`.github/workflows/ci.yml` runs:

- formatting and the locked Rust renderer/framework/ABI suites;
- a fresh 5,731-ID byte replay against the frozen Open UI archive and a
  separate strict audit of historical Chromium pass provenance;
- generated style, C ABI, migration, and SP13-R through SP20 closure checks;
- versioned renderer profiles, author-style inventory, and JavaScript disposition checks;
- Python porter/closure tests and the 7/7 repository audit;
- C ABI export/layout checks plus C and C++ consumers;
- application conformance and performance artifact validation.

`.github/workflows/hardening.yml` additionally runs the 36-scenario application
suite and starts real windows under Xvfb/Mesa OpenGL and a headless Weston pure
Wayland session with software presentation.

## Scheduled hardening

Weekly/manual jobs use pinned `nightly-2026-09-01` for:

- Miri checks for C opaque-handle kind, thread, destruction, and token reuse;
- Rust address and leak sanitizers at the C ABI boundary;
- C/C++ undefined-behavior sanitizer consumers;
- all five libFuzzer targets.

Failures block release even when they do not run on every pull request.

## Release workflow

`.github/workflows/release-v02.yml` is manual/tag-only. For both supported
architectures it:

1. verifies source contracts and crates.io package contents;
2. builds the SDK twice with a fixed source epoch and compares hashes;
3. builds `.deb` and `.rpm` packages in Ubuntu/Fedora environments;
4. installs each package and compiles/runs the packaged C example;
5. uploads checksums, SBOM, provenance, detached symbols, and artifacts;
6. produces keyless signatures and GitHub attestations when running from the
   protected release tag.

The workflow does not publish crates.io packages automatically. Compatible
crates must be published in the dependency order documented in
`docs/v02/packaging.md` after every final gate is recorded.

## Local source checks

```bash
cd bindings/rust
cargo fmt --all --check
cargo test --workspace --locked
cd ../..

python3 tools/release/build_v02_linux.py --verify-source
python3 tools/ffi/verify_abi.py
python3 tools/accountability/audit.py
git diff --check
```

Formatting the generated `pixel-compare` registry can exhaust rustfmt's default
stack because it is intentionally enormous. The generated file is checked by
its source generator; all handwritten packages remain subject to rustfmt.

## Evidence rules

Hosted repository-only audit confirms committed result metadata but does not
replace the full local audit with PNGs. A focused render overwrites
`summary.json`; preserve the authoritative full result unless the focused run
is intentionally replacing it. Performance results are qualifying only when
their artifact names the checked-in reference-machine profile and sets
`qualification: true`.
