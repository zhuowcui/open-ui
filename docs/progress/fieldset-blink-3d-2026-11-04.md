# A platform fieldset groove is drawn through Blink's 3D sides, 4 November 2026

The default 2px groove border of a fieldset is now painted the way
Blink's `BoxBorderPainter` paints any other 3D border. Each side is split
into an outer and an inner half, and a corner where the two sides' shades
differ is clipped with an antialiased miter. The theme's measured dark and
light grays are kept. Before, the platform fieldset had its own coverage
path, and at 1x five corner cells were replayed. The census goes from
22,440 to 22,473 exact, with 21 more comparisons improved, no losses and 3
comparisons worse.

## Finding

`aspect-ratio/fieldset-element-001` and its `-ref` at 2x had 16
mismatched pixels, all in the four corners of the fieldset's border.
`fieldset-baseline-alignment`, `multicol-span-all-fieldset-001` and `-003`
and `display-contents-dynamic-generated-content-fieldset-001-ref` showed
the same pattern at 1.25x, 1.5x and 2x.

- At the top-left and bottom-right corners both sides have the same shade
  in each half. Open UI drew each side's half as its own antialiased
  trapezoid, so the yellow background showed through the shared diagonal,
  for example (205, 205, 78) instead of (155, 155, 155) at device (40, 76)
  at 2x. Blink's `ColorsMatchAtCorner` gives such a corner a hard miter,
  which leaves no seam.
- At the top-right and bottom-left corners the shades differ. Blink clips
  each side with an antialiased miter (`ClipBorderSidePolygon`), so the
  diagonal blends the two grays. Open UI's platform path averaged them
  instead, or left a background cell, as at (447, 76).

Open UI already had a Blink-faithful path for authored 3D borders
(`paint_blink_3d_border_side`). The platform fieldset was excluded from it,
because that path shades black `currentColor` as 154 and 238, while the
platform fieldset measures 155 and 239. With the measured grays, a WSL
render of the border fixtures is exact at 1x without the corner replay.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A straight, opaque platform fieldset border takes Blink's 3D side path with the theme's dark and light grays. Rounded or translucent fieldset borders keep the platform coverage path. The 1x corner replay `paint_native_fieldset_corner_samples` and the corner gray it used are removed. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_borders`, `paint_blink_3d_border_side`, `paint_fieldset_borders`, `PLATFORM_GROOVE_SHADES`) |

The unit test `platform_fieldset_groove_corners_follow_blinks_3d_sides_at_2x`
paints the default groove over yellow at 2x. It checks that the top-left
diagonal is the dark gray and that the top-right corner cell is gray, with
no background in it. Without the change the test fails, with
(205, 205, 78) at the top-left.

## Measurement

This is a paired expanded census in WSL. The base is `cc62ca61`, whose
`bindings` tree equals `main` after the
[repeat scanline tie](repeat-scanline-tie-2026-11-03.md) change. The head
is `fba061f9`. The later commit `28289ae9` only adds the unit test. Its
`bindings` tree equals this change rebased onto `main` at `8e24a603`.
Both are built with the CI `SKIA_GN_ARGS` and share one Chromium cache.
Chromium is identical in all 23,728 comparisons. Open UI's output changed
in 57, none of them at 1x.

| Result | Comparisons |
| --- | --- |
| Exact | 22,440 to 22,473 |
| Gained, 33 | `fieldset-baseline-alignment` and `-ref`, `aspect-ratio/fieldset-element-001` and `-002` with their `-ref`s, `multicol-span-all-fieldset-001` and `-003` with their `-ref`s, `display-contents-dynamic-generated-content-fieldset-001-ref`, `line-clamp/webkit-line-clamp-027` and `scroll-markers/scroll-marker-group-011` and its `-ref`, each at some of 1.25x, 1.5x and 2x, and `multicol-span-all-fieldset-002-ref` at 2x |
| Improved, 21 | For example `aspect-ratio/fieldset-element-001-ref` at 1.25x (268 to 3), `contain-intrinsic-size-014` at 1.25x (551 to 72), `multicol-span-all-fieldset-001` and `-ref` at 1.25x (20 to 2) and `position-absolute-semi-replaced-stretch-other` at 1.5x (4,182 to 3,894) |
| Worse, 3 | `aspect-ratio/fieldset-element-001`, `-002` and `-002-ref` at 1.25x (269 to 398) |
| Lost | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/fieldset-blink-3d-census-v1.json).

### Remaining failures

- The 3 worse comparisons are a fieldset 10px tall under its legend. At
  1.25x its bottom border row is now one level darker than Chromium's,
  194 against 195 in 144 cells, where the platform path matched. The 253
  cells of the legend's last row, 102 against 127, are unchanged. The
  cause of the one-level difference was not found.
- `multicol-span-all-fieldset-002` keeps its column and legend residuals.
- Rounded or translucent default fieldset borders keep the platform
  coverage path. Their corners were not measured.
- The column fragment clip residuals in `multicol-rule-002`,
  `multicol-rule-fraction-003` and `multicol-span-all-block-sibling-003`
  and the 1 pixel left in `multicol-span-all-004` at 1.5x are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,440 to 22,473 exact; 33 gained, 21 improved, 3 worse, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,343 passed, 0 failed, 2 ignored, at `28289ae9` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,505 passed, 0 failed, 21 ignored, at `28289ae9` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
