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

## Verification

All results are local measurements of the branch head `23fe1b57`, except the
hosted CI rows.

| Check | Command | Result |
| --- | --- | --- |
| Guard binary, Windows | `cargo test --locked -p openui-engine --test native_resize_control_clip` | 9/9 pass, including the repaired guard and the new exact-frame test |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,395 passed, 3 failed, 24 ignored. The failures are the documented arrow and WOFF tests |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,402 passed, 1 failed, 13 ignored. The failure is the toolchain-dependent `source_layer_raster` arrow test, which also fails on `main` |
| Hosted `windows-msvc` | PR CI | 9,271 passed, 3 failed (the same documented tests), 24 ignored; every other step passes |
| Hosted `rust-parity` | PR CI | Stops at the pre-existing dependency audit (RUSTSEC-2026-0370 to 0372), as on `main` |

## Paired renderer census

The pinned Chromium toolchain behind the historical totals is not available on
this host, so the census pairs `main` (`ad491392`) with the branch under one
toolchain: system clang 21 in WSL, with the hosted CI Skia defines. Each source
was built with `tools/qualification/build_renderer.py` from a clean comparison
clone under `out/census-*`, with library provenance verified. Both were then
measured with `tools/qualification/run_renderer_matrix.py`, sharing one cache
of fresh Chromium 147.0.7727.50 captures. The
[census evidence](evidence/native-scrollbar-frame-clip-census-v1.json)
records the identities, report digests and paired deltas.

| Suite | `main` exact | Branch exact | Native images changed | Errors |
| --- | ---: | ---: | ---: | ---: |
| Expanded (original plus additions, four profiles) | 21,663 / 23,728 | 21,663 / 23,728 | 0 | 0 |
| Focused (40 profiles) | 640 / 640 | 640 / 640 | 0 | 0 |
| Primitive (40 profiles) | 960 / 960 | 960 / 960 | 0 | 0 |

Chromium pixels are identical in every pair, and no comparison gains, loses,
improves or worsens. The census corpus contains no forced scrollbar picture
that this repair reaches, so it neither regresses nor qualifies the change.
The 50 frame-clip states are its oracle. These same-toolchain counts are not
comparable to the historical census measured with the pinned toolchain, and
no renderer or release qualification is claimed.


## Correction, 21 October 2026

The `source_layer_raster` arrow failure recorded here is not toolchain-dependent.
Hosted and portable builds omitted Skia's `-DSK_ENABLE_LEGACY_SHADERCONTEXT`,
which the pinned parity profile sets. With it the test passes on Linux and
Windows; see [the Skia configuration record](skia-legacy-shader-context-2026-10-21.md).

The paired census above built both sources without that define; it is a
same-configuration pairing and its zero-change result stands.

## Update, 24 October 2026

Windows Chrome-for-Testing 147.0.7727.50 reproduces the residual pixels
exactly (row 57 of Auto width 16 at 1.25 is `#fdfdfd`). Varying the box on
that browser rules out fractional edge coverage as the cause:

| Probe at 1.25 | Observation |
| --- | --- |
| Auto width 16, bottom edge at 57.25, 57.5 and 57.75 | Row 57 is `#fdfdfd` (253) in all three |
| Auto widths 1, 1.2 and 1.4, right edge at 41.25, 41.5 and 41.75 | Column 41 is fully `#fcfcfc` (252) in all three |
| Thin width 1 | Pixel (40, 57) is 253; column 41 is white |

Neither edge value depends on where the box edge falls within the pixel, so
they do not come from an antialiased clip. Chromium's theme paints tracks and
corners as whole-pixel rectangles without antialiasing
(`ui/native_theme/native_theme_fluent.cc`, `PaintScrollbarTrack`,
`PaintScrollbarCorner`). The values therefore come from how the scrollbar
layers are rounded and composited. Reproducing them needs a model of
Chromium's scrollbar layer compositing. Capturing the layer command log with
DevTools `LayerTree` was not possible: headless software compositing emits no
`layerTreeDidChange`. The four states stay listed in
`CHROMIUM_FRAME_RESIDUALS`.

Auto width 1 at 1.25 is now exact; the other three remain. See
[the two-axis clip record](scrollbar-two-axis-clip-2026-10-24.md).
