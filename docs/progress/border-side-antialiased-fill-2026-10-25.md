# Antialiased fill for mixed-style solid border sides, 25 October 2026

A survey of near misses in the 25 October census picked out the
`ttwf-css3background-border-style` tests. At 1× they differ by two corner
pixels, but at 1.25× and 1.5× a whole row along the solid top border's inner
edge differs.

## Finding

The box has a 5 px border with solid, dotted, dashed and double sides. At
1.25× the solid top side's inner edge falls at device row 31.25. Chrome leaves
row 31 white; Open UI painted it at 25% coverage. At 1.5× the edge falls at
37.5: Chrome paints row 37 at 50% coverage, while Open UI painted it fully.

Blink (`third_party/blink/renderer/core/paint/box_border_painter.cc`, tag
`147.0.7727.50`) paints each side of a mixed-style border separately:

- `DrawSolidBoxSide` always fills the side rectangle **antialiased**.
- When a corner needs a hard miter, `PaintOneBorderSide` first clips the side
  to its polygon without antialiasing (`ComputeMiter`,
  `MitersRequireClipping`, `ClipBorderSidePolygon`). A solid side next to the
  double or dotted side of the same colour needs one.

The polygon's outer and inner edges therefore cut at pixel centres. A
half-covered row keeps its 50% coverage, and a quarter-covered row is dropped.

Open UI clipped the side the same way but filled it without antialiasing.
Then, at fractional scales, it replayed fractional coverage over every solid
side. That kept quarter rows Chromium drops and lost half rows Chromium keeps.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The solid side of a mixed-style border fills antialiased under its existing miter clip | `openui-paint` | `painter.rs` `paint_border_side_path` |
| The fractional-coverage replay of solid sides is removed | `openui-paint` | `painter.rs` `paint_borders` |

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`cf40a2b7` and `d85cd626`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,701 to 21,701 |
| Open UI images changed | 9 of 23,728, all improved |
| Lost, worsened, gained | none |

The nine improvements are the `ttwf-css3background-border-style` family at
1.25× and 1.5×. `-shorthand-missing-bottom` at 1.5×, for example, goes from 472
to 20 pixels; `-border-style` at 1.25× goes from 196 to 8. One more is
`background-clip_clip-border-area-ref` at 1.25×, from 467 to 216. The remaining
pixels are antialiasing at the dotted side's corners. Chromium is identical
in every comparison. Rows are in the
[evidence](evidence/border-side-antialiased-fill-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,701 exact before and after; 9 improved, 0 worse |
| Paint unit tests | `cargo test --locked -p openui-paint --lib` | 252 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
