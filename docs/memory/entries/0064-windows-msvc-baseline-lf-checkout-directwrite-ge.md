---
id: 0064
title: Windows MSVC baseline: LF checkout, DirectWrite generics and the Linux-oracle ledger
tags: windows, platform, workflow, fonts, ci
status: active
created: 2026-10-19
updated: 2026-10-19
refs: docs/progress/windows-baseline-2026-10-19.md, docs/DEVELOPMENT.md, docs/v02/generated/windows-pending-tests.txt, tools/ci/check_platform_scoped_tests.py
---

On 19 October 2026 the user directed Rust, C ABI and tooling work to run on Windows x64 MSVC while pinned Chromium runs on Linux in WSL (Chrome-for-Testing 147.0.7727.50 under ~/openui-oracle in WSL; its chrome binary sha256 equals the historical pin). The user chose native DirectWrite text with a per-platform oracle: Linux Open UI against Linux CfT, Windows Open UI against Windows CfT 147.0.7727.50 (not yet captured). Win32 windowing is later separate work. Gotchas: system git sets core.autocrlf=true, so .gitattributes forces LF; clone auxiliary sources with git -c core.autocrlf=false; set PYTHONUTF8=1; generators must hash as_posix() paths and write exact bytes. rust-skia builds Windows Skia without FreeType, so DirectWrite cannot load WOFF/WOFF2 and has no CSS generic aliases (openui-text maps generics to Chromium's Windows defaults). Windows main threads default to 1 MiB stacks; pixel_compare and C consumers link 8 MiB. Tests whose expected values come from Linux FreeType/Fontconfig carry cfg_attr(not(target_os = "linux"), ignore = "linux-oracle: ...") and must equal the ledger; platform-independent failures stay failing, never ledgered.
