---
id: 0068
title: Win32 desktop runtime shares the winit runtime behind the windows feature
tags: windows, platform, winit, ffi
status: active
created: 2026-10-21
updated: 2026-10-21
refs: docs/progress/windows-desktop-runtime-2026-10-21.md, bindings/rust/openui-platform/src/desktop.rs, tools/ffi/verify_native_window.py
---

Since 21 October 2026 openui-platform/src/desktop.rs (formerly linux.rs) is the shared winit runtime for Linux (linux feature) and Windows (windows feature); desktop gates are any(all(feature="linux", target_os="linux"), all(feature="windows", target_os="windows")) across openui-platform, openui, openui-ffi and the examples. Windows uses glutin WGL, softbuffer Win32, accesskit_winit's UI Automation adapter, and a Win32 CF_UNICODETEXT clipboard on windows-sys (clipboard-win is BSL-1.0, outside the release license policy). Verify with tools/ffi/verify_native_window.py --protocol win32. After any Cargo.lock change run tools/release/generate_v02_contract.py, because dependency-policy.json pins the lockfile hash.
