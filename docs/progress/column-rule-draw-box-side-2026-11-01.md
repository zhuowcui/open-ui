# Solid and 3D column rules as Blink's DrawBoxSide, 1 November 2026

A solid, groove, ridge, inset or outset column rule is now painted as
Blink's `BoxFragmentPainter::PaintColumnRules` paints it: one antialiased
rect in CSS pixels under the device scale for a solid rule, and one per
integer half for a groove or ridge rule. The census stays at 22,308 exact,
with 2 comparisons improved, no losses and no comparison worse.

## Finding

Chrome 147.0.7727.50 paints column rules through the legacy
`BoxFragmentPainter::PaintColumnRules` (`core/paint/box_fragment_painter.cc`),
since `CSSGapDecoration` is still `experimental` in
`runtime_enabled_features.json5`. It maps the rule style through
`ComputedStyle::CollapsedBorderStyle`, so inset becomes ridge and outset
becomes groove. It then passes the pixel-snapped rule to
`BoxBorderPainter::DrawBoxSide` as a left side, or a top side in a vertical
writing mode. `DrawBoxSide` calls `DrawLineForBoxSide` with no adjacent
widths. A solid rule is then one antialiased `FillRect`
(`DrawSolidBoxSide`). A groove or ridge rule is split at
`(x1 + x2 + 1) / 2` in integer CSS pixels, and each half is one antialiased
rect in its own shade (`DrawRidgeOrGrooveBoxSide`).

Open UI painted a solid rule as a rect of device-pixel coverage, and a 3D
rule through its border side painter, which shades the two halves from
device-pixel rects. In `multicol-rule-inset-000` at 1.25x the rule's half
boundary falls at 281.25 device pixels. Chrome's pixel (281, 50) is
(0, 144, 0): the light half covers a quarter of it and the dark half the
rest. Open UI's was (0, 145, 0).

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A solid, groove or ridge column rule, after the inset and outset mapping, is drawn by the port of `DrawLineForBoxSide` from the integer rule rect, as a left side, or a top side in a vertical writing mode, with no adjacent widths. | `openui-paint` | `painter.rs` `paint_column_rule` |

Dotted, dashed and double rules keep the border side painter.

The unit test `inset_column_rule_paints_blinks_ridge_halves_at_fractional_scale`
paints the 10px lime inset rule at 1.25x and checks three pixels across it
against Chrome's capture. It fails without the change: the half boundary
pixel (31, 50) is then (0, 145, 0) instead of (0, 144, 0).

## Measurement

This is a paired expanded census in WSL. The base is `446e554f`, whose
rendering equals `main` at `facd898f`. The head is `a79e9af1`. Both use one
shared Chromium cache, and Chromium is identical in every comparison.

| Result | Comparisons |
| --- | --- |
| Exact | 22,308 to 22,308 |
| Gained | none |
| Improved, 2 | at 1.25x: `multicol-rule-inset-000` and `multicol-rule-ridge-000`, 868 to 806 |
| Lost, worsened | none |

The rows are in the
[census evidence](evidence/column-rule-draw-box-side-census-v1.json).

### Remaining failures

- The `multicol-rule-*-000` comparisons at 1.25x keep between 186 and
  1,240 mismatched pixels, including `multicol-rule-none-000`, which paints
  no rule. In the three examined, `-none-000`, `-solid-000` and
  `-ridge-000-ref`, every mismatched pixel is one level off Chrome, where an
  Ahem glyph is drawn over a pixel that a background or border edge covers
  by half: Chrome's (63, 63, 63) is Open UI's (64, 64, 64) at the bottom
  edge of the black boxes, and Chrome's (0, 64, 0) is Open UI's (0, 65, 0)
  at a half-covered rule edge under a glyph. These comparisons use
  `chromium-linux-lcd` text. The cause was not established in this change.
  The owner is `openui-paint` (`text_painter.rs`).
- `multicol-rule-color-001-ref` and `multicol-rule-fraction-001-ref` at
  1.25x and 1.5x, and `multicol-rule-002` and `multicol-rule-fraction-003`
  at 1.5x, were not examined.
- Dotted, dashed and double column rules still use the border side painter.
  `multicol-rule-dotted-000` keeps 3 mismatched pixels at 1.5x and 6 at 2x.
  The owner is `openui-paint`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,308 to 22,308 exact; 0 gained, 2 improved, 0 worsened, 0 losses |
| Other raster configurations | rerender of every comparison in `default`, `deterministic-alias`, `deterministic-alias-subpixel` and `chromium-linux-lcd` | the same 2 improvements, in `chromium-linux-lcd`, at `a79e9af1`; no mismatch count changed in `default`, `deterministic-alias` or `deterministic-alias-subpixel` |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint` | 8,335 passed, 0 failed, 2 ignored, at `a79e9af1` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,498 passed, 0 failed, 20 ignored, at `a79e9af1` |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
