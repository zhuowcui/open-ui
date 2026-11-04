# A native text field's face stays inside its rounded ring, 4 November 2026

The default white face of a native text field or menu list is now filled
on the same rounded rect as its 1px ring, inset by half the border, as
`NativeThemeBase::PaintTextField` does. Before, Open UI filled the whole
snapped box, so at scaled devices the face covered the backdrop in the
corner cells outside the ring. The census goes from 22,480 to 22,487
exact, with 6 more comparisons improved. No comparison is lost or worse.

## Finding

`floats-wrap-bfc-outside-001` and its `-ref` at 2x kept 4 mismatched
pixels each, all at the outer corners of a default textarea over a lime
backdrop. At the top left corner Chromium keeps lime in the five device
cells outside the ring's 2px arc, (0,0), (1,0), (2,0), (0,1) and (0,2)
from the box origin. Open UI painted them white. `stretch-input-in-column` and
`select-element-zero-height-001` also draw default text controls, and
this change makes them exact at 2x.

`ThemePainterDefault::PaintTextField` passes `zoom = EffectiveZoom()`,
which is 1 in the census because the device scale is a canvas transform.
`NativeThemeBase::PaintTextField` insets the rect by `border_width / 2`,
uses `radius = AdjustBorderRadiusByZoom(2)` and draws the fill and the
stroke on those same bounds with default, non-antialiased flags. So the
face's corners are cut by the same 2px CSS arc as the ring, half a CSS
pixel inside the box.

An author background on a text control is drawn by the antialiased
coverage-blended contour path instead, which keeps its full fill.
Insetting that path too regressed `select-element-zero-height-002` at
1.5x from 52 to 5,704 mismatched pixels in an experiment, so the inset is
only applied to the non-antialiased default face.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The non-antialiased default face of a native text control is filled on the ring's rect, inset by half its 1px border, with the ring's 2px radius. The antialiased author face keeps the full snapped rect. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`paint_direct_native_text_control_theme`) |

The unit test `native_text_field_face_stays_inside_its_rounded_ring_at_2x`
paints a 20x16 default field over lime at 2x. It checks that the five
corner cells outside the arc stay lime, that the ring is
(118, 118, 118) at the arc and that the face is white inside it. It fails
on the old fill, which made (12, 10) white.

## Measurement

This is a paired expanded census in WSL. The base is `9bebead6`, the head
of the [partial row run](hline-run-spans-2026-11-04.md) census. Its
rendering equals `main` at `964854c6`, which only adds documentation. The
head is `4fda9cf8`, this change on `main` at `964854c6`. Both are built
with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 13.

| Result | Comparisons |
| --- | --- |
| Exact | 22,480 to 22,487 |
| Gained, 7 | `stretch-input-in-column` at 1.25x (7 to 0), 1.5x (24 to 0) and 2x (24 to 0); `floats-wrap-bfc-outside-001` and its `-ref` at 2x (4 to 0); `select-element-zero-height-001` and its `-ref` at 2x (2 to 0) |
| Improved, 6 | `floats-wrap-bfc-outside-001` and its `-ref` at 1.25x (3 to 2) and 1.5x (3 to 1); `select-element-zero-height-001` and its `-ref` at 1.5x (3 to 2) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/native-text-field-face-census-v1.json).

### Remaining failures

- `floats-wrap-bfc-outside-001` and its `-ref` keep 2 pixels at 1.25x
  and 1 at 1.5x, and `select-element-zero-height-001` and its `-ref`
  keep 2 at 1.5x. These are corner cells of the same field where the
  fractional device scale puts the arc between pixel centres. They were
  not diagnosed further.
- The antialiased author face is unchanged.
- `border-radius-clip-002` keeps its rounded-rect antialiasing residue.
- The `background-repeat-round-2` and `-3` residuals are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,480 to 22,487 exact; 7 gained, 6 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,345 passed, 0 failed, 2 ignored, at `4fda9cf8` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,507 passed, 0 failed, 21 ignored, at `4fda9cf8` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
