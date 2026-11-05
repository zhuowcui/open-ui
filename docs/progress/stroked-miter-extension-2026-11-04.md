# A stroked side's clip runs past its one mitered corner, 4 November 2026

A dotted, dashed or double border side whose two corner miters differ is
now clipped once per mitered corner, and each clip runs 0.1px past its own
mitered end, as `BoxBorderPainter::ClipBorderSidePolygon` does. Before,
Open UI clipped such a side to its unextended edge quad. The census goes
from 22,487 to 22,492 exact, with 4 more comparisons improved. No
comparison is lost or worse.

## Finding

`ttwf-css3background-border-style`, its `-shorthand` and its
`-shorthand-missing-left` draw a 5px black border on a 160x160 box: solid
top, dotted right, dashed bottom and double left. At 1x each kept 2
mismatched pixels in the last dot of the dotted right side, just above
the bottom right corner.

The dotted right side is painted before the solid top, which will
overdraw it, so `ComputeMiter` gives the top corner no miter. The bottom
corner joins a dashed side, and a dotted and dashed pair requires a hard
miter. With two different miters, `ClipBorderSidePolygon` does not clip
the edge quad directly. It clips once for each corner that has a miter,
using the side's bounding box at the other end and extending the mitered
end by `kExtensionLength`, 0.1px, outward along the side. That extension
moves the clip's device bounds, and Skia's dashing and the coverage of
the endpoint dot follow those bounds. Open UI's unextended clip left the
dot one coverage level off in four rows.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| When a dotted, dashed or double side's start and end miters differ, the clip polygon's mitered ends are moved 0.1px outward along the side, matching `kExtensionLength` in `ClipBorderSidePolygon`. Equal miters and solid and 3D sides are unchanged. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_border_side_path`, `stroked_miters`) |

The unit test `dotted_side_clip_runs_past_its_one_mitered_corner` paints
the same border at (20, 20), 160x160, on a 200x200 white surface. It checks
column 179 from row 175 to 179 against Chromium's levels, 177, 35, 13, 72
and 0. It fails on the old clip.

## Measurement

This is a paired expanded census in WSL. The base is `4fda9cf8`, the head
of the [text field face](native-text-field-face-2026-11-04.md) census.
Its rendering equals `main` at `86de930a`, which only adds documentation.
The head is `7a33d7dd`, this change on `main` at `86de930a`. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 9.

| Result | Comparisons |
| --- | --- |
| Exact | 22,487 to 22,492 |
| Gained, 5 | `ttwf-css3background-border-style`, its `-shorthand` and its `-shorthand-missing-left` at 1x (2 to 0); `-shorthand-missing-left` at 1.25x (1 to 0) and 1.5x (3 to 0) |
| Improved, 4 | `ttwf-css3background-border-style` and its `-shorthand` at 1.25x (6 to 5) and 1.5x (6 to 3) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/stroked-miter-extension-census-v1.json).
Before the census, a batch of the 1,204 expanded cases whose source uses a
dotted, dashed or double border gave the same 9 changes and none worse.

### Remaining failures

- `ttwf-css3background-border-style` and its `-shorthand` keep 5 pixels at
  1.25x and 3 at 1.5x. They are in the double left side's partial column
  near the bottom left corner, which `-shorthand-missing-left` does not
  draw. Chromium has (128, 192) where Open UI has (127, 191), at
  (27, 220) to (27, 222) and (28, 220) to (28, 221) at 1.25x and at
  (34, 263) to (34, 265) at 1.5x. The owner is `openui-paint` double side
  painting. It was not diagnosed further.
- `border-radius-clip-002` keeps its rounded-rect antialiasing residue.
- The `background-repeat-round-2` and `-3` residuals are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,487 to 22,492 exact; 5 gained, 4 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,346 passed, 0 failed, 2 ignored, at `7a33d7dd` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,508 passed, 0 failed, 21 ignored, at `7a33d7dd` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
