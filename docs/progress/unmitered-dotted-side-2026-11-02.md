# Unclipped unmitered dotted, dashed and double sides, 2 November 2026

A dotted, dashed or double border side whose corners both have no miter is
no longer clipped to its side polygon. This matches Blink's
`BoxBorderPainter::MitersRequireClipping`. The census goes from 22,323 to
22,335 exact. No comparison is lost or worse.

## Finding

At 1.25x after the [glyph mask change](glyph-mask-a8-gamma-2026-11-02.md),
`multicol-rule-dotted-000` still had 50 mismatched pixels and its `-ref`
had 100. The dotted column rule itself was already exact. Every remaining
pixel was in the reference span's `border-left: lime dotted 0.2em`, a 10px
dotted side with no other border.

Chrome draws a thick dotted side as round-cap, zero-length dashes
(`StyledStrokeData`, `GraphicsContext::DrawLineWithStyle`). Each round cap
reaches past the side's inner edge by half a pixel's coverage. Open UI
clipped the side with an antialiased clip at its inner edge, x = 230 in CSS
pixels. At 1.25x that clip covers device column 287 by half, which halved
the cap's coverage there. A clip in the path also changes how Skia
rasterizes the dots inside it.

Blink clips a side only when it needs to. `PaintOneBorderSide` sets
`should_clip = MitersRequireClipping(miter1, miter2, style)`, which is true
for a hard miter at either corner, or for any miter at all on a dotted,
dashed or double side (`StyleRequiresClipPolygon`). `ComputeMiter` gives a
corner no miter when the adjacent edge has no used width, or when that edge
will overdraw the corner (`WillOverdraw`). A lone dotted side therefore has
no miter at either corner and is drawn without a clip.

The same rule covers more cases:

- `ttwf-css3background-border-style-shorthand-missing-bottom` has dotted
  left and right sides between solid top and bottom sides of the same colour.
  The solid sides overdraw the corners, so the dotted sides have no miter.
- `parallel-flow-trailing-margin-003-crash` has a lone 100px dashed top side.
- `multicol-loads-indefinitely-001-crash` has lone dotted top sides.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `paint_borders` passes `clip_to_miter = false` for a dotted, dashed or double side whose two corner miters are both `BorderMiter::None`. Every other side keeps its existing clip decision. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_borders`, `side_clips`) |

The unit test `unmitered_dotted_side_is_not_clipped_to_its_inner_edge`
(`painter.rs`) paints a 10px lime dotted left border at 1.25x over a black
box, in the reference span's geometry. It checks five pixels against
the dot coverage that Chrome shows. Device column 37, the half-covered
column at the inner edge, is (0, 111, 0) in the test. With the clip it is
(0, 56, 0). Chrome's capture shows 55 there because the span's Ahem glyph
covers that pixel by half. The test draws no glyph, so it checks the dot's
own coverage.

## Measurement

This is a paired expanded census in WSL. The base is `a03dcd38`, whose
rendering equals `main` at `ef36fd1c`. The head is `580272cb`. Both are
built with `-USK_GAMMA_APPLY_TO_A8` and share one Chromium cache. Chromium
is identical in all 23,728 comparisons. Open UI's output changed in 12.

| Result | Comparisons |
| --- | --- |
| Exact | 22,323 to 22,335 |
| Gained, 12 | `multicol-rule-dotted-000` and its `-ref` at 1.25x, 1.5x and 2x; `ttwf-css3background-border-style-shorthand-missing-bottom` at 1.25x, 1.5x and 2x; `multicol-loads-indefinitely-001-crash` at 1.25x and 1.5x; `parallel-flow-trailing-margin-003-crash` at 1.25x |
| Improved, lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/unmitered-dotted-side-census-v1.json).

### Remaining failures

- `multicol-rule-color-001-ref` and `multicol-rule-fraction-001-ref` at
  1.25x and 1.5x, and `multicol-rule-002` and `multicol-rule-fraction-003`
  at 1.5x, still have 192 to 500 mismatched pixels in `deterministic-alias`.
  An image edge that covers a pixel by half is one level high. The cause
  was not established. The owner is `openui-paint`.
- `multicol-span-all-block-sibling-003` still has 2 mismatched pixels at 2x.
  They were not examined.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,323 to 22,335 exact; 12 gained, 0 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,337 passed, 0 failed, 2 ignored, at `580272cb` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,499 passed, 0 failed, 21 ignored, at `580272cb` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
