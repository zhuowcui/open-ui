# Scrollbar compositing model, 25 October 2026

The [two-axis clip record](scrollbar-two-axis-clip-2026-10-24.md) left three
pinned Chromium scrollbar states inexact. It attributed them to scrollbar
layer compositing that pixel probes could not explain. DevTools tracing gave
direct evidence, and Open UI now models it. All 50 pinned frames are exact.

## Evidence

Windows Chrome-for-Testing 147.0.7727.50 runs the same Blink as the pinned
Linux build. A trace with the `disabled-by-default-cc.debug`,
`cc.debug.quads`, `viz.quads` and `cc.debug.display_items` categories records:

- every cc layer, with its bounds, compositing reasons and raster scale;
- each layer's display list, with the clip and draw operations.

`tools/qualification/trace_chrome_layers.py` prints both for a page.
Pixels come from viz's `SoftwareRenderer`. Layer space is CSS pixels,
rasterized at the device scale. The Chromium sources cited below are read at
tag `147.0.7727.50`.

Three findings:

1. **Blink places scrollbars in integer CSS geometry.** Frames are laid out in
   `PixelSnappedBorderBoxSize` at the rounded paint offset
   (`PaintLayerScrollableArea::RectForVerticalScrollbar`,
   `RectForHorizontalScrollbar`, `CornerRect`, and
   `ScrollableAreaPainter::PaintScrollbar`). The vertical bar ends at the far
   edge and the horizontal bar at the bottom edge. When a bar is thicker than
   the box, the bar leaves the box on the start or top side. The traces
   record a 1 px Thin box's bar as `clipRect(-9,0,1,…)`, and the horizontal
   bar of an Auto 16×14 box at y −1 to 14. Open UI anchored bars at the
   clamped client edge instead, and used unsnapped sizes.
2. **The overflow-controls clip is antialiased when the controls are painted
   into the page layer.** Blink creates `OverflowControlsClip` only when the
   control frames leave the snapped border box (`NeedsOverflowControlsClip`).
   Its clip rect is snapped (`ToSnappedClipRect`). The page layer replays it
   as an antialiased `clipRect`. A non-antialiased fill that rounds into a
   half-covered pixel is then blended: Auto width 16 at 1.25 gets row 57
   `#fdfdfd`.
3. **Composited scrollers put their controls in an opaque tile layer.** The
   layer's bounds are the snapped controls rectangle. It rasterizes into
   `ceil(bounds × scale)` texels. Partially covered edge texels take the layer
   colour (`RasterSource::ClearForOpaqueRaster`, or a solid colour quad), and
   the clip becomes an enclosing scissor. This is Auto width 15 at 1.75
   painting column 82, at edge 82.25.

When Chromium composites, measured over sizes and all five scales:

| Condition | Composited |
| --- | --- |
| No Blink overflow: `ScrollWidth().Round() > (width − borders − bar).Round()` fails in both axes | never |
| Overflow, device scale ≥ 1.5 (`WidgetBase::ComputeLCDTextPreference` ignores LCD text) | always |
| Overflow, device scale < 1.5, empty scroll area | always: an opaque background trivially covers empty contents (`PaintArtifactCompositor::ComputeNeedsCompositedScrolling`) |
| Overflow, device scale < 1.5, non-empty scroll area | never |

So 16×14 Auto is composited at 1.5, 1.75 and 2, but not at 1 or 1.25. Box
width 15.2 at height 14 follows the same split.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Element scrollbar frames use Blink's integer geometry, anchored at the far edge; viewport scrollbars keep client anchoring | `openui-paint` | `painter.rs` `native_scrollbar_frame`, `ElementScrollbarPlacement` |
| The thumb and the scrollable-path corner are placed in those frames | `openui-paint` | `painter.rs` `paint_native_scrollbars` |
| `scroller_is_composited` decides compositing as in the table above | `openui-paint` | `painter.rs` |
| With protruding frames: composited scrollers fill the enclosing device rectangle with the track colour under an enclosing clip; others use the snapped clip, antialiased | `openui-paint` | `painter.rs` `paint_element_scrollbars` |
| The pinned-frame test requires 50 exact frames and has no residual list; unit tests pin the compositing table and frame placement | tests | `native_resize_control_clip.rs`; `painter.rs` `overflow_controls_frame_guards` |

The [two-axis rule](scrollbar-two-axis-clip-2026-10-24.md) approximated the
composited case. It is replaced.

## Measurement

A probe grid of empty `overflow:scroll` boxes compares against pinned Linux
Chrome 147.0.7727.50 in WSL. It covers Auto and Thin, 16 widths from 1 to 30,
and 10 heights from 1 to 60, including fractional sizes, at every census
scale and 1.75: 1,600 cases.
`tools/qualification/capture_scrollbar_probe_grid.py` captures the frames.

| Scale | `main` exact | This change |
| --- | --- | --- |
| 1 | 246 / 320 | 320 / 320 |
| 1.25 | 169 / 320 | 320 / 320 |
| 1.5 | 230 / 320 | 320 / 320 |
| 1.75 | 175 / 320 | 320 / 320 |
| 2 | 230 / 320 | 320 / 320 |
| Total | 1,050 | 1,600 |

No probe worsens. At 1×, `main`'s misses are bars anchored at the client edge.
Per-probe hashes and counts are in the
[evidence](evidence/scrollbar-compositing-model-v1.json).

Windows Chrome draws Thin bars' arrows, and larger Auto bars' arrows,
differently from Linux. Measured against Windows Chrome, every remaining
difference is in arrow pixels where Windows and Linux Chrome disagree.

| Check | Before | After |
| --- | --- | --- |
| Pinned Linux frames | 47 of 50 exact | 50 of 50 exact |
| Paired expanded census, Linux | 21,701 exact | 21,701 exact; no Open UI image changes |

The census corpus contains no forced-scrollbar picture that the model
reaches, so it confirms no regression but does not qualify the change.

## Remaining

Composited scrollers whose frames stay inside the box are not modelled. Those
are ordinary scrollers at 1.5× and above. cc fills their bar layers'
partially covered edge texels the same way. With integer CSS geometry this
changes pixels only at device scales whose fractions are not halves, such as
1.75, which no census profile uses. The layer colour is taken as the track
colour; a page that styles the corner differently is unmeasured.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Clip test binary | `cargo test --locked -p openui-engine --test native_resize_control_clip` | 9 pass |
| Overflow-controls unit tests | `cargo test --locked -p openui-paint --lib overflow_controls` | 10 pass |
| Paired expanded census, WSL | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, `501d27dc` against `cf40a2b7` | 21,701 to 21,701 exact; 0 Open UI images change in 23,728 comparisons; Chromium identical ([evidence](evidence/scrollbar-compositing-model-census-v1.json)) |
| Probe tool tests | `python -m unittest tools.qualification.test_scrollbar_probe_tools` | 6 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
