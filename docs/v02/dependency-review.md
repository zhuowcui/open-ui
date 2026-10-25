# Dependency review

Public runtime dependencies added for Open UI v0.2 are pinned in
`bindings/rust/Cargo.lock` and must be stable releases compatible with Rust
1.85 or newer. `generate_v02_contract.py` rejects prerelease packages and
freezes the lockfile digest.

## Required security check

`tools/release/check_dependency_security.py` audits both
`bindings/rust/Cargo.lock` and `bindings/rust/fuzz/Cargo.lock`. The existing
`rust-parity` CI job and release source-contract job run it. Every SDK, Debian
and RPM build also runs it, including local `--allow-dirty` and prebuilt-library
builds. The generated dependency-policy digest check alone does not inspect
advisories, licenses or transitive compiler requirements.

The checker requires cargo-audit **0.22.2**, installed separately with Rust
**1.94.1**. The tool's own [manifest requires Rust 1.88](https://github.com/RustSec/rustsec/blob/281452c35cf0870969042374110f099a411bc185/cargo-audit/Cargo.toml);
this does not change the framework's Rust 1.85 floor or release compiler.

```bash
cargo +1.94.1 install cargo-audit --version =0.22.2 --locked \
  --root /tmp/openui-audit-tool
python3 tools/release/check_dependency_security.py \
  --cargo-audit /tmp/openui-audit-tool/bin/cargo-audit \
  --output /tmp/openui-dependency-security
```

Use a new output directory outside the checkout. The checker uses an isolated
explicit configuration, a freshly fetched official RustSec database, no ignored
advisories or target/severity filters, and `--deny warnings`. It accepts only
the dated, reviewed `unmaintained` notices in its `REVIEWED_EXEMPTIONS` list,
matched by advisory and crate. Expired or unused exemptions fail; see
[the zbus review](../progress/dependency-audit-zbus-exemption-2026-10-25.md). It also fetches
the official sparse registry entry for every locked crates.io version and
checks its checksum and yank status. This separate completeness check is
required because cargo-audit can omit yank checks after an index-fetch error.
Network errors, malformed or partial results, findings, changed source bytes or
changed lockfile bytes/mtimes all fail. No dependency is updated automatically.

Raw tool output, process status, tool hash, database revision, registry response
bytes and before/after source identities are retained in the output directory.
CI uploads that evidence on failure as well as success. Security evidence is a
current measurement; a passing generated-policy check is not a passing audit.
Standalone vendor lockfiles describe inactive vendor build graphs; dependencies
used by the framework and fuzz workspace are audited through their active
lockfiles. A new standalone release graph requires explicit admission.

## Dependency inventory

| Package | Version | License | Purpose and maintenance review |
|---|---:|---|---|
| accesskit | 0.25.0 | MIT OR Apache-2.0 | Canonical cross-platform semantic node, tree-update, and action vocabulary. Stable release from the maintained AccessKit project; its Rust 1.85 MSRV matches Open UI. |
| accesskit_winit | 0.34.0 | Apache-2.0 | Maintained AccessKit adapter for winit and the Unix AT-SPI bridge; stable and Rust 1.85 compatible. |
| winit | 0.30.13 | Apache-2.0 | Maintained X11/Wayland event-loop and window abstraction; stable with an MSRV below Open UI's. |
| glutin | 0.32.3 | Apache-2.0 | Maintained EGL/GLX context and surface abstraction; stable with an MSRV below Open UI's. |
| glutin-winit | 0.5.0 | MIT | Maintained winit bootstrap integration for choosing native EGL/GLX configurations; stable with an MSRV below Open UI's. |
| gl | 0.14.0 | Apache-2.0 | Khronos-registry-derived OpenGL function loader used by the texture presenter. |
| softbuffer | 0.4.8 | MIT OR Apache-2.0 | Maintained CPU framebuffer presentation on X11 and Wayland; stable with an MSRV below Open UI's. |
| smithay-clipboard | 0.7.3 | MIT | Wayland seat data-device clipboard ownership without an XWayland dependency; stable and Rust 1.85 compatible. |
| x11-clipboard | 0.9.3 | MIT | X11 CLIPBOARD/PRIMARY selection implementation over x11rb. |

Transitive package `uuid` 1.26.1 is used by AccessKit tree identifiers and is
covered by the same lockfile, license, MSRV, and advisory checks. Scheduled CI
compiles the public headless and Linux framework surfaces with Rust 1.85; tag
CI repeats that compiler floor while producing both native SDK architectures.

The isolated fuzz workspace pins `libfuzzer-sys` 0.4.13 under its combined
MIT/Apache-2.0 and NCSA license. It is maintained by the Rust Fuzz project,
requires a nightly compiler only for fuzz execution, and is excluded from the
runtime workspace and all release artifacts.

All eleven crates.io packages declare Apache-2.0, repository/homepage metadata,
the Rust 1.85 floor, and version 0.2.0. Internal dependencies carry both a
local path and an exact registry version. The non-publishable `openui-ffi`
crate is packaged as the native SDK after the same dependency review.

## XML dependency backport and publication gate

The Linux accessibility graph uses `zbus_xml` 5.1.0 through
`zbus-lockstep` 0.5.2 and AccessKit. Its upstream `quick-xml` 0.38 requirement
selects a version affected by RUSTSEC-2026-0194 and RUSTSEC-2026-0195. The
patched `quick-xml` 0.41 line supports Rust 1.79, while newer published
`zbus_xml` releases require Rust 1.87. Open UI retains its Rust 1.85 floor.

The workspace therefore applies `vendor/zbus_xml-v5_1`: the official MIT
licensed 5.1.0 crate with only its normalized `Cargo.toml` dependency
requirement changed from `quick-xml` 0.38 to 0.41. Its Rust source, tests,
license and original manifest remain byte-identical to the upstream archive.
`OPENUI-BACKPORT.json` records the official archive checksum, each original
and retained file checksum, the manifest change, and the canonical retained
source inventory digest. The upstream standalone lockfile is omitted because
the active production graph is governed by the framework lockfile.

The production lock also selects `crossbeam-epoch` 0.9.20, which fixes
RUSTSEC-2026-0204 and retains its Rust 1.61 minimum. Fuzz has a separate lock;
both active locks remain subject to the complete strict security check.
Dependency resolution and source review do not replace Rust 1.85 compilation,
Linux feature checks, upstream XML round-trip/error guards or a fresh audit.

Cargo only honors `[patch]` at the final consuming workspace root;
[dependency manifests do not propagate patches](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section).
This local backport does not make the registry copy of `zbus_xml` 5.1.0 patched.
The SDK ships the verified backport source and license, identifies it in the
SBOM, and adds an explicit root patch to its Rust example manifests. Standalone
crates.io consumer publication remains blocked until a compatible fixed
upstream graph is published or an actual consumer explicitly applies and
qualifies the backport. Publishing an Open UI crate alone cannot satisfy that
gate.
