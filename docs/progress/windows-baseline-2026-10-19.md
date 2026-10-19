# Windows x64 MSVC development baseline, 19 October 2026

At the user's direction, the Rust workspace, C ABI and Python tooling now build
and run on Windows. Pinned Chromium stays on Linux, in WSL. This is an
experimental development baseline, not a v0.2 platform promise; renderer,
native API and release qualification remain open as recorded in
[current status](current-status.md).

## Host and toolchain

| Item | Value |
| --- | --- |
| Host | Windows 11 Pro 10.0.26300, Intel Core i9-12900KF (AVX2/FMA, no AVX-512) |
| Rust | 1.94.1 `x86_64-pc-windows-msvc` |
| C/C++ | Visual Studio 2022 Build Tools, LLVM 23.1.3 clang and clang-cl |
| Python | 3.12.10, Pillow 12.2.0, clang-format 18.1.8, `PYTHONUTF8=1` |
| Skia | rust-skia `a31b86ba` source build, same `SKIA_GN_ARGS` defines as Linux CI |
| Linux oracle | Chrome-for-Testing 147.0.7727.50 in WSL Ubuntu 26.04; `chrome` SHA-256 `0ba87684…f08848` equals the historical pin |

Commands and environment are in [DEVELOPMENT.md](../DEVELOPMENT.md#windows-x64-msvc).

## Accepted changes

| Change | Owner | Source |
| --- | --- | --- |
| LF checkout for every text file; vendored crates and 20 historical CRLF/mixed CSVs keep stored bytes; renormalization changes no blob | repository | `.gitattributes` |
| Portable Python discovery and 8 MiB main-thread stack for `pixel_compare` | `pixel-compare` | `build.rs` |
| Linux window, presentation and clipboard dependencies are target-gated | `openui-platform` | `Cargo.toml` |
| CSS generic families resolve to Chromium's Windows default families when DirectWrite has no alias; Linux keeps Fontconfig | `openui-text` | `font/collection.rs` |
| POSIX relative paths, exact-byte writes, portable locks and process groups in generators and tools | tooling | `tools/qualification`, `tools/release`, `tools/wpt`, `tools/accountability/audit.py` |
| C ABI verification reads COFF exports and builds consumers against the DLL import library with an 8 MiB stack; portable thread header for seven guard consumers | `openui-ffi` tooling | `tools/ffi/verify_abi.py`, `examples/c_v02/portable_thread.h` |
| Windows-pending Linux-oracle ledger and checker | CI | `tools/ci/check_platform_scoped_tests.py`, `docs/v02/generated/windows-pending-tests.txt` |
| `windows-msvc` CI job; Linux job also checks the ledger | CI | `.github/workflows/ci.yml` |

Before the generic-family change, every fallback list on Windows was empty
because DirectWrite does not resolve `sans-serif`, so no text shaped or painted.
The Windows aliases (Arial, Times New Roman, Consolas, Comic Sans MS, Impact,
Segoe UI, Cambria Math, Segoe UI Emoji) follow Chromium's Windows font settings
and remain unqualified until compared with Windows Chrome-for-Testing.

## Verification on Windows

All results below are local measurements of this branch on the host above.

| Check | Command | Result |
| --- | --- | --- |
| Workspace build | `cargo build --workspace --locked` | Pass, 7 min 57 s with a fresh Skia build |
| Workspace tests before ledger markers | `cargo test --workspace --locked --no-fail-fast` | 9,393 passed, 15 failed, 13 ignored across 225 test and doctest groups; exit 101 |
| Affected targets after markers | the nine failing targets, rerun | 4 failed; the 11 Linux-oracle tests report `ignored, linux-oracle: …` |
| C ABI | `python tools/ffi/verify_abi.py --library <target>/debug/openui_ffi.dll --keep-going` | 164 exports exact; 41 C and 35 C++ consumers pass |
| Python suite (CI's 17 modules) | `python -m unittest …` | 300 tests pass, 6 skipped |
| Ledger checker | `python -m unittest tools.ci.test_check_platform_scoped_tests`; `python tools/ci/check_platform_scoped_tests.py` | 6 tests pass; 11 recorded, ledger exact |
| Byte-exact ledgers | every `--check` step of the Python CI job, `build_v02_linux.py --verify-source`, `audit.py --repository-only` | Pass |
| Rust formatting | `cargo fmt --all --check` | Pass |

The 13 ignored tests are the same pre-existing ignores as on Linux. The six
Python skips are five Linux Chrome capture lifecycle tests and one SP17 splice
test whose generated modules need a rustfmt main-thread stack larger than the
fixed 1 MiB Windows default.

## Remaining Windows failures

These are failures, not ledger entries; their expected values do not depend on
the Linux oracle.

| Test | Owner | Observation |
| --- | --- | --- |
| `openui-engine::native_resize_control_clip::actual_native_scrollbar_frames_are_clipped_independently_of_track_and_thumb_ink` | overflow-control paint | Same known failure as Linux: ink at physical row 57 for scale 1.25 |
| `openui-engine::source_layer_raster::original_edge_and_all_eight_controls_keep_real_property_boundaries` | overflow-control paint / Skia build | Viewport scrollbar arrows differ from the pinned Chromium frame at 1.25×, 1.5× and 1.75×: 23, 26 and 32 pixels per mode, 1–8 levels each, in the arrow columns. Linux matches. Cause not yet identified |
| `openui-text::font::collection::tests::pinned_wpt_font_containers_report_identical_owned_metadata` | `openui-text` font registration | WOFF registration returns `MalformedFont`: rust-skia builds Windows Skia without FreeType and DirectWrite cannot load WOFF/WOFF2 |
| `openui-ffi::tests::c_font_metadata_matches_pinned_multiformat_fixtures` | `openui-ffi` / `openui-text` | Same WOFF cause through the C ABI (`InvalidArgument`) |

The WOFF repair, decoding web font containers before DirectWrite as Chromium's
OTS does, and Windows text qualification against Windows Chrome-for-Testing
147.0.7727.50 belong to the DirectWrite text task. A Win32 window, input and
presentation runtime is separate later work.

## Linux-oracle ledger

Eleven tests are ignored outside Linux and listed in
[windows-pending-tests.txt](../v02/generated/windows-pending-tests.txt): four
depend on FreeType's fractional advance of loaded Ahem, five on FreeType Ahem
or DejaVu metrics, and two on Fontconfig fallback face selection. They compile
and run unchanged on Linux.
