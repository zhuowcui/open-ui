# Vendored fonts (pixel-parity)

These fonts are pinned in-process by `openui-text`'s `FontCache` so OpenUI
resolves them deterministically, independent of ambient system font config.
This lets WPT pixel comparisons render byte-identical glyph outlines to the
headless-Chromium reference.

- `Ahem.ttf` — the canonical Web Platform Tests Ahem font (exact 1em square
  glyphs; used for deterministic single-line text parity, SP14).
  sha256: b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448
  Must stay byte-identical to the Ahem the Chromium reference renders.

- `DroidSansFallback-reduced.ttf` — Chromium writing-mode test fixture for
  CJK and fullwidth upright glyphs.
  sha256: 27db42b79d0846f6fd01b3d6a8233df9a8a5ece80b042299dc4174c48213ffd3
  Source: Chromium `third_party/blink/web_tests/fast/writing-mode/resources/`.
- `NotoSansDevanagari-Regular.ttf` — Chromium WPT fixture for representative
  complex-script shaping.
  sha256: b1dffa1fccb30dc45287111834a9db15c652b05d4d67201abe73e67717017590
  Source: Chromium `third_party/blink/web_tests/external/wpt/fonts/noto/`.
- `NotoColorEmoji.ttf` — Chromium Noto Color Emoji v2.051 fixture for emoji
  fallback and cluster painting.
  sha256: 72a635cb3d2f3524c51620cdde406b217204e8a6a06c6a096ff8ed4b5fd6e27b
  Source: Chromium `third_party/blink/web_tests/third_party/NotoColorEmoji/`.

`DroidSansFallback-reduced.ttf` and `NotoSansDevanagari-Regular.ttf` are
distributed under Apache-2.0 (`LICENSE-Apache-2.0.txt`). Noto Color Emoji is
distributed under OFL-1.1 (`LICENSE-OFL-1.1.txt`).

- `DejaVuSans.ttf` — DejaVu Sans Book 2.37.
  sha256: ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280
- `DejaVuSans-Bold.ttf` — DejaVu Sans Bold 2.37.
  sha256: 5c1247acef7f2b8522a31742c76d6adcb5569bacc0be7ceaa4dc39dd252ce895
- `DejaVuSansMono.ttf` — DejaVu Sans Mono Book 2.37.
  sha256: c805f9436dbc268644c1d9584f01a601a653e028e08fd74b9b949f6cf8304d88
- `DejaVuSansMono-Bold.ttf` — DejaVu Sans Mono Bold 2.37.
  sha256: 3a3c502eeff669a231549e80df9f7c49de109bafe303170409e905d0b31a38fe
- `DejaVuSerif.ttf` — DejaVu Serif Book 2.37.
  sha256: 8f2c103bfa3fd5de71f1b92b18f21906b5a26871fb7e19a9a4c9af539c3cc7ab
- `DejaVuSerif-Bold.ttf` — DejaVu Serif Bold 2.37.
  sha256: 847b33e13925f19ff87e4d934d6b3cf7cac35ce16424f6f670e40c2f377cf2df

SP16 resolves the CSS `sans-serif`, `monospace`, and `serif` generics to
these exact assets on both renderers. The license is recorded in
`LICENSE-DejaVu.txt`.
