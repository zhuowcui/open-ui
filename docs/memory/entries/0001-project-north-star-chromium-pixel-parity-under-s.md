---
id: 0001
title: Project north star: Chromium pixel parity under strict accountability
tags: project, standard, policy, native-api
status: active
created: 2026-07-08
updated: 2026-10-02
refs: docs/progress/current-status.md, docs/engineering-principles.md, docs/v02/supported-platforms.md, docs/renderer/contract.md
---

Open UI is a standalone native framework with Rust DOM/style/layout/paint.
Chromium is the sole pixel reference, with zero tolerance. Historical Open UI
screenshots are provenance, not expected pixels or a compatibility gate.
Unexplained pixel differences remain bugs; completion requires measured
evidence, an accountability audit, and reviewed ownership for every residual.
Open UI never executes JavaScript, in this or future versions. Applications implement interaction with public native Rust methods and Rust callbacks over the retained engine. Any needed browser-style element behavior must be implemented in Rust and callable through the consuming app API. An internal Engine method, a test fixture, or a WPT JavaScript exclusion does not complete or waive that API work.
