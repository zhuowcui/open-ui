---
id: 0071
title: Windows UI Automation check drives the accessibility example
tags: windows, accessibility, uia, accesskit, ci
status: active
created: 2026-10-21
updated: 2026-10-21
refs: tools/platform/verify_windows_uia.ps1, bindings/rust/examples/accessibility, docs/progress/windows-ui-automation-2026-10-21.md
---

Since 21 October 2026 tools/platform/verify_windows_uia.ps1 (pwsh, .NET UIAutomationClient) launches the accessibility example built with --features windows and checks framework 'Open UI', named Button/CheckBox/Edit, SetFocus, Invoke (button text 'Pressed N times' becomes its UIA name), Toggle, ValuePattern.SetValue and WindowPattern.Close with exit 0. winit creates an extra unnamed 'Winit Thread Event Target' window per process; pick the titled window. Building the example without the windows feature produces a headless binary that only renders a PNG into the working directory. Runs in hosted windows-msvc CI.
