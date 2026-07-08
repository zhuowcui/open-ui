# Vendored fonts (pixel-parity)

These fonts are pinned in-process by `openui-text`'s `FontCache` so OpenUI
resolves them deterministically, independent of ambient system font config.
This lets WPT pixel comparisons render byte-identical glyph outlines to the
headless-Chromium reference.

- `Ahem.ttf` — the canonical Web Platform Tests Ahem font (exact 1em square
  glyphs; used for deterministic single-line text parity, SP14).
  sha256: b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448
  Must stay byte-identical to the Ahem the Chromium reference renders.
