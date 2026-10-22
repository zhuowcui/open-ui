---
id: 0072
title: Windows SDK archive: build_v02_windows.py with a find_package smoke
tags: windows, release, packaging, cmake, ffi
status: active
created: 2026-10-21
updated: 2026-10-21
refs: tools/release/build_v02_windows.py, tools/release/test_windows_sdk.py, docs/progress/windows-sdk-package-2026-10-21.md
---

Since 21 October 2026 tools/release/build_v02_windows.py builds openui-sdk-0.2.0-x86_64-pc-windows-msvc.zip (bin/openui_ffi.dll, lib/openui_ffi.lib renamed from openui_ffi.dll.lib, debug/openui_ffi.pdb, include/openui, lib/cmake/OpenUI with IMPORTED_IMPLIB and OpenUI_RUNTIME_DLL, share/openui examples/licenses/SBOM). --smoke extracts it and builds examples/c hello.c (plus its .h files) with CMake+Ninja+clang-cl, copying the DLL next to the exe. It requires a clean tree unless --allow-dirty and reuses build_v02_linux helpers. Deterministic: sorted entries, fixed ZIP times from SOURCE_DATE_EPOCH (min 1980). Not a v0.2 release artifact; no signing or installer.
