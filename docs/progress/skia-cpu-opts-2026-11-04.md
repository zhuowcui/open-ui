# Skia's CPU-specific raster stages are installed as Chromium installs them, 4 November 2026

Open UI now calls `SkGraphics::Init` before it paints or rasterizes, as
Chromium does at startup. Skia then runs its AVX2 and FMA raster stages on
a CPU that has them, as Chromium's renderer does, instead of the baseline
SSE stages. A spaced background pattern's shader phase is also reduced to
the first tile at or after the paint rect's origin, as
`BackgroundImageGeometry::ComputePhase` does. The census goes from 22,492
to 22,516 exact, with 30 more comparisons improved. No comparison is lost
or worse.

## Finding

Chromium calls `SkGraphics::Init` while the renderer starts. Among other
things it installs Skia's CPU-specific raster pipeline stages. On an
x86-64 CPU with AVX2 and FMA these are the `hsw` stages, which evaluate
gradients and bilinear sampling with fused multiply-adds. Open UI never
called it, so Skia kept its baseline stages. The two round differently
where a value sits on a rounding boundary. Many background residuals were
a single level off at one or two pixels: `background-image-centered`
and the `background-repeat-round` and `-space` cases at scaled devices, for
example. skia-safe exposes the call as `skia_safe::graphics::init`.

With the FMA stages in place, the first paired census had one comparison
worse: `background-repeat_gradient-repeat-spaced-with-borders` at 1.5x,
which went from 377 to 729 mismatched pixels. Its spaced gradient tiles
repeat beneath a border. Open UI placed the pattern shader's origin at the
positioning area's origin, but Blink sets the tiling phase to the paint
rect's origin plus the tile offset reduced modulo one tile step
(`BackgroundImageGeometry::ComputePhase`, used by
`GeneratedImage::DrawPattern`). The two origins are a whole step apart
and tile identically. A tile edge's half-pixel bilinear sample still gets
different weights from the two phases, and under the FMA stages the edge
pixel rounded to the other level. With the reduced phase the case goes to
369 at 1.5x, below its base, and to exact at 1.25x.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `initialize_skia` calls `skia_safe::graphics::init` once. Painting, rasterizing and the pixel-compare harness call it before drawing. The paint crate's unit test binary calls it when it loads, so no test result depends on test order. | `openui-paint` | `bindings/rust/openui-paint/src/lib.rs` (`initialize_skia`), `bindings/rust/openui-paint/src/painter.rs` (`paint_fragment`), `bindings/rust/openui-paint/src/render.rs` (`rasterize_picture`), `bindings/rust/pixel-compare/src/main.rs` |
| A spaced pattern's shader origin moves by whole tile steps to the first tile at or after the paint rect's origin, in each direction that repeats. | `openui-paint` | `bindings/rust/openui-paint/src/painter.rs` (`reduced_pattern_phase`) |

The unit test `repeating_radial_gradient_rounds_as_skia_fma_stages_do`
paints `repeating-radial-gradient(blue, green 20px)` in a 200x150 box. It
checks two pixels whose dithered value sits on a rounding boundary
against Chromium's levels. It runs only on a CPU with AVX2 and FMA, and it
fails without the initialization. The unit test
`spaced_pattern_phase_is_reduced_to_the_paint_rect_origin` checks the
phase reduction, including non-repeating and zero-step axes.

## Measurement

This is a paired expanded census in WSL. The base is `7a33d7dd`, the head
of the [stroked miter](stroked-miter-extension-2026-11-04.md) census. The
head is `88a9ba2b`, this change on `main` at `a85d00b9`. Both are built
with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium is
identical in all 23,728 comparisons. Open UI's output changed in 54.

| Result | Comparisons |
| --- | --- |
| Exact | 22,492 to 22,516 |
| Gained, 24 | `background-image-centered` and its `-ref` at 1x, 1.25x, 1.5x and 2x; `scrollbar-gutter-with-background-gradient-ref` at 1.25x (114 to 0) and 1.5x (612 to 0); `background-repeat-round-2` and its `-ref` at 1.5x; `background-repeat-round-3` and its `-ref` at 1.25x and 1.5x; `background-attachment-fixed-inside-transform-1` and its `-ref` at 1.5x; `background-repeat-space-1-ref`, `-3-ref`, `-4-ref` and `-5-ref` at 2x; `background-image-gradient-currentcolor-radial-repaint-ref` at 2x; `background-repeat_gradient-repeat-spaced-with-borders` at 1.25x |
| Improved, 30 | `background-repeat-round-1-ref` and `-1a` to `-1e` at 1.25x (6 to 4); `background-repeat-space-1-ref`, `-1a` to `-1c`, and `-3` to `-7` with their refs at 1.25x; `background-repeat-space-1a` to `-1c` and `-3` to `-8` at 1.5x, for example `-space-8` (392 to 42); `background-repeat_gradient-repeat-spaced-with-borders` at 1.5x (377 to 369) |
| Lost, worsened | none |

The census covers every raster configuration, including `default`,
`deterministic-alias` and `deterministic-alias-subpixel`. The rows are in
the [census evidence](evidence/skia-cpu-opts-census-v1.json). The
initialization alone, at `5dab158e`, gave 22,515 exact with one comparison
worse. Before the second census, a batch of the 204 expanded comparisons
whose name contains `space` gave only the two changes in
`gradient-repeat-spaced-with-borders`. The second census agrees: against
the first, it changes only those two comparisons.

### Remaining failures

- `background-repeat_gradient-repeat-spaced-with-borders` keeps 369 pixels
  at 1.5x. Most are in column and row 232, the pattern's right and bottom
  edge at 232.5 device px. Chromium paints partial blue and green there,
  for example (207, 219, 243), where Open UI leaves white. The owner is
  `openui-paint` background painting, in the spaced pattern's paint
  extent. It was not diagnosed further.
- `background-repeat-round-1-ref` and `-1a` to `-1e` keep 4 pixels at
  1.25x. `background-repeat-round-2` and its `-ref` keep 4 at 2x, and
  `-round-3` and its `-ref` keep 2 at 2x. The owner is `openui-paint`
  background painting.
- `ttwf-css3background-border-style` and its `-shorthand` keep their
  double-side pixels, 5 at 1.25x and 3 at 1.5x.
- `border-radius-clip-002` keeps its 2 pixels of rounded-rect
  antialiasing residue at 1x.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,492 to 22,516 exact; 24 gained, 30 improved, 0 worsened, 0 losses |
| Linux text, layout and paint crates in WSL | `cargo test --locked --no-fail-fast -p openui-text -p openui-layout -p openui-paint`, with the CI `SKIA_GN_ARGS` | 8,348 passed, 0 failed, 2 ignored, at `88a9ba2b` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,510 passed, 0 failed, 21 ignored, at `88a9ba2b` |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Format | `cargo fmt --all --check` | passes |
