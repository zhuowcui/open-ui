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

The SDK archive contains:

```text
include/openui/             generated C headers
lib/                        static/shared libraries
lib/pkgconfig/              relocatable openui.pc
lib/cmake/OpenUI/           relocatable CMake package
debug/                      detached shared-library symbols
share/openui/examples/      C and Rust examples
share/openui/licenses/      project and bundled-font notices
share/openui/openui.spdx.json
share/openui/BUILD-METADATA.json
```

Every binary artifact has a `.sha256` sidecar. One in-toto JSONL statement
records the Git revision, target, source epoch, and artifact digests. Tag CI
adds keyless signatures and GitHub artifact attestations; a local development
build is intentionally unsigned.

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
