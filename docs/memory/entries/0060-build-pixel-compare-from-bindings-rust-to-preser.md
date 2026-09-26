---
id: 0060
title: Build pixel_compare from bindings/rust to preserve Chromium Skia parity
tags: build, skia, pixel-compare, gotcha, chromium, rustfmt
status: active
created: 2026-09-01
updated: 2026-09-01
refs: bindings/rust/.cargo/config.toml, bindings/rust/target/release/build/skia-bindings-c18ad339f750fe8f/output
---

Run Cargo from bindings/rust so its nested .cargo/config.toml injects the pinned Chromium sysroot and Skia flags; repo-root Cargo builds cause raster-only mismatches. For cargo fmt on the generated WPT registry, first run 'ulimit -s 262144' because rustfmt can overflow its default main-thread stack while still surfacing a misleading zero status.
