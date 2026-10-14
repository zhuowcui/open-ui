# Linux packaging

Open UI v0.2 produces native SDKs for
`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`. Release builds use
ordinary system build tools and do not use the optional Chromium parity config.
The driver requires a clean worktree; `--allow-dirty` exists only for local
packaging tests and does not produce release-qualified evidence.

## Build inputs

- Rust 1.85 or newer with the requested GNU target.
- Clang or GCC, CMake/Ninja dependencies required by rust-skia, pkg-config,
  FreeType, Fontconfig, and OpenGL development headers.
- `objcopy`, `strip`, and `readelf` from binutils.
- `dpkg-deb` for Debian packages or `rpmbuild` for RPM packages.
- A clean source revision and the checked-in `Cargo.lock`.
- cargo-audit 0.22.2 installed with the separate Rust 1.94.1 toolchain, as
  described in [dependency review](dependency-review.md#required-security-check).
  Set `OPENUI_CARGO_AUDIT` to its executable path, or put it on `PATH`.

Build an SDK and Debian package:

```bash
SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD) \
python3 tools/release/build_v02_linux.py \
  --target x86_64-unknown-linux-gnu \
  --format sdk --format deb
```

On Fedora, replace `deb` with `rpm`. `--library-dir` accepts a directory with
prebuilt `libopenui_ffi.a` and `libopenui_ffi.so`; it exists for package
verification and never changes the artifact layout.

Every actual build performs the strict shared dependency security check before
creating artifacts. `--allow-dirty` and `--library-dir` do not bypass it.
`--verify-source` remains the generated-input and locked-metadata check without
an advisory audit; release CI runs the security checker separately before it. Fresh audit evidence
is kept outside the checkout under `OPENUI_DEPENDENCY_SECURITY_EVIDENCE_ROOT`
(the system temporary directory by default). A failed or incomplete audit stops
packaging and retains its diagnostics. Audit evidence is separate from the
deterministic package contents.

The SDK archive contains:

```text
include/openui/             generated C headers
lib/                        static/shared libraries
lib/pkgconfig/              relocatable openui.pc
lib/cmake/OpenUI/           relocatable CMake package
debug/                      detached shared-library symbols
share/openui/examples/      C and Rust examples
share/openui/licenses/      project and bundled-font notices
share/openui/sources/backports/ verified local dependency backport sources
share/openui/DEPENDENCY-BACKPORTS.json
share/openui/openui.spdx.json
share/openui/BUILD-METADATA.json
```

Every binary artifact has a `.sha256` sidecar. One in-toto JSONL statement
records the Git revision, target, source epoch, and artifact digests. Tag CI
adds keyless signatures and GitHub artifact attestations; a local development
build is intentionally unsigned.

Required local backports are declared in the workspace root's
`workspace.metadata.openui.dependency-backports` table. Packaging requires the
exact declared package, source path, license, provenance, source inventory and
upstream archive identity in the resolved Linux graph. Missing provenance or a
symlink in the original manifest/source path stops the build before path
canonicalization. Other preexisting vendors remain supported. Discovered local
backports with `OPENUI-BACKPORT.json` are also checked against their complete
file inventories before packaging. The SDK includes
their source, original license and provenance. The SPDX package checksum
identifies the canonical retained source inventory; `sourceInfo` separately
records the unmodified upstream archive checksum and provenance checksum.
Missing, changed, extra or symlinked source files stop the build. The copied
source and notices receive the same deterministic metadata normalization as
the SDK.
Source verification and SDK metadata resolve `openui-ffi/linux`, matching the
native library build and including its accessibility dependency graph.

Packaged Rust example manifests explicitly apply these sources at the example
workspace root. Independent Rust consumers must supply the same root patch or
use a compatible upstream fixed dependency graph; a patch in an Open UI
dependency manifest does not propagate. The standalone registry consumer
publication gate in [dependency review](dependency-review.md#xml-dependency-backport-and-publication-gate)
remains required.

## Consumer checks

With an extracted SDK:

```bash
export PKG_CONFIG_PATH="$PWD/openui-sdk-0.2.0-x86_64-unknown-linux-gnu/lib/pkgconfig"
cc examples/c/hello.c $(pkg-config --cflags --libs openui) -o hello
LD_LIBRARY_PATH="$PWD/openui-sdk-0.2.0-x86_64-unknown-linux-gnu/lib" ./hello
```

CMake consumers use `find_package(OpenUI 0.2 CONFIG REQUIRED)` and link
`OpenUI::OpenUI`.

## Reproducibility

The release driver normalizes ownership, permissions, mtimes, gzip headers,
tar order, Cargo codegen settings, and package timestamps. CI builds each SDK
twice in isolated directories and compares SHA-256 values before upload.
Publication is blocked if the two outputs differ, a dynamic dependency names
Chromium/Blink, generated files drift, or a package install/example test fails.

## Crates.io publication order

All supported crates carry both a local path and exact `=0.2.0` registry
version. Cargo deliberately cannot fully package a dependent crate until that
version of its Open UI prerequisites exists in the registry. Inspect every
package with `cargo package --list`, then publish and verify in this order:

1. `openui-geometry`;
2. `openui-style` and `openui-platform`;
3. `openui-dom`, `openui-text`, and `openui-macros`;
4. `openui-layout`;
5. `openui-paint`;
6. `openui-compositor`;
7. `openui-engine`;
8. `openui`.

Wait for registry propagation and run `cargo package --locked` before advancing
each level. `openui-ffi` is distributed only in the native SDK, not crates.io.
