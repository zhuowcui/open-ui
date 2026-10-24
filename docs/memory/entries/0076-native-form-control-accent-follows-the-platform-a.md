---
id: 0076
title: Native form-control accent follows the platform accent (Windows DWM)
tags: windows, native-controls, paint, accent, qualification
status: active
created: 2026-10-24
updated: 2026-10-24
refs: bindings/rust/openui-dom/src/tree.rs, bindings/rust/openui-engine/src/lib.rs, bindings/rust/openui-platform/src/lib.rs, bindings/rust/pixel-compare/src/main.rs, tools/qualification/compare_windows_text.py, docs/progress/windows-form-control-accent-2026-10-24.md
---

Chromium's native theme uses the OS accent (user_color) for checked checkbox/radio and range progress; Windows reports HKCU\Software\Microsoft\Windows\DWM\AccentColor (0xAABBGGRR). Document::form_control_accent() returns the platform accent or Chromium's default (0,117,255); Engine::set_platform_accent_color sets it (paint invalidation); openui_platform::system_accent_color() reads DWM on Windows and App::run_document applies it; pixel_compare takes --accent-color RRGGBB; compare_windows_text.py passes the host accent by default and records accent_color. Derived shades (e.g. thumb edge = 50% accent + (118,118,118)) follow automatically. Windows: 10,369 -> 10,399 exact (+30, 0 losses). The fieldset 156/240 vs 155/239 greys were the default black-currentColor groove theme shades (PLATFORM_GROOVE_SHADES in painter.rs: Windows 156/198/240, Linux 155/197/239), fixed 24 October 2026 (+15 exact, 0 losses).
