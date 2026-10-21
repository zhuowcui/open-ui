# Native scrollbar frame clip repair, 20 October 2026

This is the next bounded task from the
[17 October checkpoint](branch-checkpoint-2026-10-17.md#next-bounded-task):
the failing guard
`openui-engine::native_resize_control_clip::actual_native_scrollbar_frames_are_clipped_independently_of_track_and_thumb_ink`
(changed ink at physical row 57 for scale 1.25).

## Independent Chromium oracle

`tools/qualification/capture_native_scrollbar_oracle.py` renders the guard's
states as empty ordinary Divs in pinned Chrome-for-Testing 147.0.7727.50. The
states are Auto and Thin forced scrollbars on 14px-high boxes of width 0, 1,
14, 15 and 16 at 32,32 in a 160×160 viewport, at scales 1, 1.25, 1.5, 1.75
and 2. Each state is captured twice, and with `--historical` the tool requires
byte equality with an earlier capture. Run in WSL Ubuntu 26.04 against the
Chrome binary whose SHA-256 equals the historical pin (`0ba87684…f08848`):

```sh
python3 tools/qualification/capture_native_scrollbar_oracle.py \
  --chrome ~/openui-oracle/chrome/linux-147.0.7727.50/chrome-linux64/chrome \
  --output ~/openui-oracle/scrollbar-capture-v3 \
  --crops bindings/rust/openui-engine/tests/native_scrollbar_frame_chromium_oracles \
  --historical /mnt/d/openui-v02-qualification-d174ea0b/native-control-ordinary-div-chromium-runtime-v4754
```

All 50 frames repeat exactly and match the historical frames byte for byte.
The [receipt](evidence/native-scrollbar-frame-clip-chromium-v1.json) records
every digest, ink extent and crop. The 40 non-empty crops are each the box's
enclosing physical rectangle, and every pixel outside them is verified white.

## Findings

- **Guard contract.** Chromium's ink exceeds the guard's half-open
  pixel-centre border bounds in 27 of the 40 non-empty states, including row
  57 at 1.25. The pixel-centre rule is not Chromium's behavior.
- **Layout is unchanged.** A CDP geometry probe of the same pinned build
  reports client and child sizes of 85×65 (Auto) and 90×70 (Thin), and
  85.25×65.5 for a fractional box, at every scale. Scrollbar thickness stays
  15 or 10 CSS px in layout, as Open UI already does.
- **Paint was wrong.** For forced bars without a scroll range, Chromium
  records ordinary pictures whose track, corner and arrow parts round to
  physical pixels independently. For example, Thin width 16 at 1.25 paints
  its vertical bar at [48,60)×[40,45), its horizontal bar at [40,48)×[45,58),
  and its corner at [48,60)×[45,58). Open UI rasterized the picture offscreen
  at `ceil(size × scale)` and resampled it at fractional origins. That blurred
  the arrows (`#c4c4c4` and `#a7a7a7` where Chromium has `#8b8b8b`) and let
  Thin bars bleed one pixel past the box.

## Accepted changes

| Change | Owner | Source |
| --- | --- | --- |
| Forced-scrollbar pictures rasterize their parts directly under the device transform, without antialiasing | overflow-control paint | `openui-paint/src/painter.rs` `paint_fluent_scrollbar_picture` |
| The frame-clip guard requires changed ink inside Chromium's measured extent for the same state | Engine test | `openui-engine/tests/native_resize_control_clip.rs` |
| New exact test compares full frames with the 46 matching Chromium frames | Engine test | `forced_native_scrollbar_pictures_match_pinned_chromium_frames` |

Against the 50 Chromium frames, native exactness rises from 40 to 46. The
resize-grip guards keep their unchanged pixel-centre rule; this oracle does
not measure grips.

## Remaining differences

These four states remain inexact. Their native ink lies inside Chromium's
extent, and they are listed in `CHROMIUM_FRAME_RESIDUALS`:

| State | Difference |
| --- | --- |
| Auto width 1 at 1.25 | Chromium paints column 41 (`#fcfcfc`, the enclosing right edge of a 41.25 box); native stops at 41 |
| Auto width 15 at 1.75 | Chromium paints column 82 to the enclosing edge of 82.25; native stops at 82 |
| Auto width 16 at 1.25 | Chromium partially covers row 57 (`#fdfdfd`) at the 57.5 bottom edge; native paints `#fcfcfc` |
| Thin width 1 at 1.25 | One pixel (40, 57): `#fdfdfd` in Chromium, `#fcfcfc` native |

They concern how the compositor clips or covers fractional box edges, which
this paint model does not reproduce. Ownership: overflow-control paint and
compositor clip.
