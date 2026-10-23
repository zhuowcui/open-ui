---
id: 0066
title: Dependency audit is its own CI job; zbus 5 unmaintained notices stay open
tags: ci, dependencies, security, zbus, accesskit
status: active
created: 2026-10-21
updated: 2026-10-22
refs: docs/CI.md, .github/workflows/ci.yml, tools/release/check_dependency_security.py
---

On 21 October 2026 the user chose to split the reviewed-lockfile audit out of rust-parity into its own dependency-security job rather than port AccessKit's AT-SPI chain or accept exemptions. Since 16 October 2026 RustSec marks zvariant, zbus_names and zvariant_derive (RUSTSEC-2026-0370..0372) unmaintained because they merged into zbus 6.0; accesskit_unix 0.24, accesskit_atspi_common 0.21 and atspi 0.30 still require zbus 5, and zbus 6 needs Rust 1.89 (MSRV is 1.85). dependency-security and the release workflow's audit keep denying warnings with no exemptions, so they stay red until upstream moves; check crates.io for accesskit_unix/atspi releases on zbus 6 before re-attempting. Rechecked 22 October 2026: the newest atspi-common 0.14 still requires zbus 5.5 and zvariant 5; the audit still reports only RUSTSEC-2026-0370..0372.
