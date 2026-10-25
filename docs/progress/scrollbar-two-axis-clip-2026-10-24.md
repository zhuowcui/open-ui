# Scrollbar two-axis edge clip, 24 October 2026

The [frame clip record](native-scrollbar-frame-clip-2026-10-20.md) left four
pinned Chromium scrollbar states inexact. Its update attributed them to
Chromium's scrollbar layer compositing. This change closes one of them,
Auto width 1 at 1.25, with a measured clip rule. It also records what the
other three need.

## Finding

Pinned Chromium frames, and Windows Chrome-for-Testing 147.0.7727.50 on
probes the pinned set does not cover, show this rule. When an empty
`overflow:scroll` box is smaller than its scrollbars in both axes, Chrome
paints every device pixel the box touches. At 1.25, a right edge at 41.25
paints column 41. When the bars stick out in one axis only, the painted
extent stops at the snapped edge, as Open UI already does.

| Probe at 1.25 | Bars stick out | Column 41 (edge 41.25) |
| --- | --- | --- |
| Thin width 1, height 5, 8 or 10 | both axes | painted |
| Thin width 1, height 14 | horizontally only | white |
| Auto width 1, height 14 | both axes | painted |
| Auto width 1, height 16, 30 or 60 | horizontally only | white |

The style is not the cause: Thin and Auto bars follow the same split.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `OverflowControlsFrameGeometry` records whether the control frames leave the border box both horizontally and vertically | `openui-paint` | `painter.rs` `OverflowControlsFrameGeometry::resolve` |
| When they do, element scrollbars clip to the enclosing device pixels of the snapped border clip; otherwise they keep the snapped clip | `openui-paint` | `painter.rs` `paint_element_scrollbars`, `OverflowControlsClipScope::new_enclosing_device` |
| The pinned-frame test expects 47 exact frames and lists three residual states | engine tests | `openui-engine/tests/native_resize_control_clip.rs` |

The clip is in recording space, which is CSS pixels. The enclosing rectangle
is taken at the document's device scale and mapped back.

## Measurement

| Check | Before | After |
| --- | --- | --- |
| Pinned Linux frames, `forced_native_scrollbar_pictures_match_pinned_chromium_frames` | 46 of 50 exact | 47 of 50 exact; no state worsens |
| Windows Chrome, Auto, height 14, 13 widths × 1.25 and 1.75 | 7 of 26 inexact | 2 of 26 inexact |
| Paired expanded census, Linux | 21,701 exact | 21,701 exact; no Open UI image changes |

Windows and Linux Chrome frames are byte-identical for every pinned Auto state
and for Thin width 1. Thin bars wide enough to draw arrows paint different
arrows on Windows, so the Windows comparison uses only arrow-free Thin cases.
Per-probe hashes and counts are in the
[evidence](evidence/scrollbar-two-axis-clip-v1.json).

## Remaining

The rule is measured, not derived from Blink. Four behaviours remain
unmodeled, and the probes do not show the rule painting a pixel that Chrome
leaves white.

1. **A `#fdfdfd` or `#fefefe` last row or column.** These cases stay inexact:
   Auto width 16 and Thin width 1 at 1.25 (pinned), Auto width 30 at
   height 1 or 14, Auto width 1 at height 30, and Auto or Thin width 3 at
   height 60. The partial pixel sits beside an otherwise uniform `#fcfcfc`
   (252) track. For Auto width 16 its value stays 253 when the bottom edge
   moves from 57.25 to 57.75. Blink's Fluent theme draws track and buttons from a stretched nine-patch
   resource (`ScrollbarThemeFluent::UsesNinePatchTrackAndButtonsResource`;
   `ScrollbarThemeAura::NinePatchTrackAndButtonsCanvasSize` and `Aperture`).
   cc samples that resource bilinearly when the scrollbar is a composited
   `PaintedScrollbarLayer` (`PaintedScrollbarLayerImpl::AppendNinePatchScaledTrackAndButtons`).
   Blink composites a scrollbar only when `MayCompositeScrollbar` holds:
   the area scrolls overflow and the scrollbar's `Maximum()` is nonzero.
   `HasHorizontalOverflow` compares an unclamped client width, which goes
   negative when the box is narrower than the vertical bar. The model needs
   that decision plus the nine-patch geometry and viz's bilinear sampling.
2. **Partial right column at 1.75 with one-axis protrusion.** At height 60,
   Chrome paints the partial column for widths 3 and 7.7 (edges 61.25 and
   69.475) but not for 1.2 and 5.4 (edges 58.1 and 65.45). At 1.25, no
   one-axis case paints it. The pinned Auto width 15 at 1.75 (edge 82.25)
   is this case.
3. **Arrow pixels.** Auto width 30 at height 14 and 1.75 differs in arrow
   greys. Windows arrows differ from Linux for Thin bars, and no pinned frame
   covers this Auto size, so this difference is unattributed.
4. **Thin width 1, height 10, at 1.25.** Chrome paints column 41. Open UI's
   CSS-pixel geometry has a 10 px thin bar in a 10 px box, which does not
   protrude. Chrome's device geometry rounds the thin thickness to 13 against
   a 12.5 px box. A device-pixel protrusion test would cover it, but no
   pinned frame exercises it.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Clip test binary | `cargo test --locked -p openui-engine --test native_resize_control_clip` | 9 pass |
| Overflow-controls guards | `cargo test --locked -p openui-paint --lib overflow_controls` | 8 pass |
| Paired expanded census, WSL | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, `fc9335e7` against `501d27dc` | 21,701 to 21,701 exact; 0 Open UI images change in 23,728 comparisons; Chromium identical ([evidence](evidence/scrollbar-two-axis-clip-census-v1.json)) |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,457 passed, 0 failed, 20 ignored |

## Update, 25 October 2026

The two-axis rule approximated Chromium's composited-scroller path, and the
three residuals are now exact. See [the compositing model record](scrollbar-compositing-model-2026-10-25.md).
