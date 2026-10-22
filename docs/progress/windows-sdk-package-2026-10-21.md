# Windows SDK archive, 21 October 2026

The Win32 runtime record left Windows packaging unverified. This change adds a
deterministic Windows x64 MSVC SDK archive and checks it by building a C
consumer from the extracted archive. Windows remains an experimental
development platform; the archive is not a v0.2 release artifact.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `build_v02_windows.py` stages headers, `bin/openui_ffi.dll`, its import library as `lib/openui_ffi.lib`, the PDB, a relocatable CMake package (`OpenUI::OpenUI` with `IMPORTED_IMPLIB` and `OpenUI_RUNTIME_DLL`), the C and Rust examples, licenses and an SPDX SBOM, then writes a deterministic zip and its SHA-256 sidecar | release tooling | `tools/release/build_v02_windows.py` |
| `--smoke` extracts the archive into a fresh directory, configures the `hello` C example with `find_package(OpenUI 0.2 REQUIRED CONFIG)`, builds it with clang-cl and runs it | release tooling | same |
| Unit tests for archive determinism, the ZIP time epoch and library discovery | release tooling | `tools/release/test_windows_sdk.py` |
| `windows-msvc` CI builds and smokes the archive from the desktop-feature DLL | CI | `.github/workflows/ci.yml` |

The builder reuses the Linux builder's source verification, dependency
backports and SBOM. The import library keeps the DLL name it binds to, so only
the file is renamed. A static library is not shipped: it would need Skia's
native dependencies.

## Verification

Local measurements on the Windows host of the
[Windows baseline](windows-baseline-2026-10-19.md).

| Check | Command | Result |
| --- | --- | --- |
| Archive and consumer smoke | `python tools/release/build_v02_windows.py --library-dir <target>/debug --output <dir> --smoke` | Pass: `hello.exe` built through `find_package(OpenUI)` from the extracted archive runs with exit code 0 |
| Determinism | two builds from the same tree and library | Byte-identical archives, SHA-256 `f3ad4756…ccd0` in the prototype run |
| Unit tests | `python -m unittest tools.release.test_windows_sdk` | 3 tests pass |

## Not covered

Code signing, an installer (MSIX or MSI), a release-optimized DLL in CI and a
Rust consumer built from the archive's examples are not part of this check.
