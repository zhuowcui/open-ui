---
title: Fidelity
description: How Open UI is measured against Chromium, the current numbers with their sources, and the known failures.
---

Open UI's rendering target is Chromium. Not a spec reading, not a design
file: the pixels that a pinned Chromium build produces for the same page. This
page explains how that comparison works, gives the current numbers with the
committed record each comes from, and lists what fails or is not measured.

:::note[What these numbers are]
They are measurements of the experimental `main` branch. They are not a
release qualification, and Open UI is not "pixel-perfect" in general: most
CSS areas, real-font text and performance are not measured yet.
:::

## How a comparison works

1. **The oracle is pinned.** Every reference image comes from Chrome for
   Testing **147.0.7727.50**, run headless with fixed fonts, flags and device
   scale. Capture receipts record the version and the binary's SHA-256.
2. **The same page goes through Open UI.** Open UI parses no HTML and runs
   no JavaScript. Offline tooling ports each test page to a native element
   tree and styles, which the engine an app uses lays out and paints with
   Open UI's CPU Skia rasterizer.
3. **Bytes are compared.** Both images have the same size, and a comparison
   is exact only if every RGBA byte matches. The tolerance is zero; there is
   no perceptual threshold or per-test allowance.
4. **Scales are part of the test.** Each test runs at four profiles:
   800×600 at 1×, 375×667 at 2×, 1280×720 at 1.25× and 1920×1080 at 1.5×.
   Fractional scales are where most antialiasing differences appear.

## The suites

| Suite | What it is | Size |
| --- | --- | --- |
| Census (expanded) | Tests ported from the [web-platform-tests](https://github.com/web-platform-tests/wpt) CSS directories below, plus 201 later additions | 5,932 tests × 4 profiles = 23,728 comparisons |
| CSS Grid coverage | The WPT `css-grid` area, kept apart from the census | 4,204 measured comparisons |
| Focused and primitive | Small raster checks across 40 profiles | 640 and 960 comparisons |
| Targeted oracles | Per-feature captures, such as 54 color swatches or 50 scrollbar frames | varies |

The census's original 5,731 tests come from these WPT areas
([manifest](https://github.com/zhuowcui/open-ui/blob/main/tools/qualification/manifests/complete-5731.json)):

| Area | Tests |
| --- | ---: |
| `css-flexbox` | 1,384 |
| `css-break` (fragmentation) | 1,220 |
| `css-backgrounds` | 845 |
| `css-multicol` | 700 |
| `css-sizing` | 586 |
| `css-overflow` | 507 |
| `css-position` | 163 |
| CSS 2 floats | 158 |
| `css-box` | 87 |
| `css-display` | 81 |

Most census tests draw text with Ahem, a test font whose glyphs are boxes, so
they measure layout and paint more than typography.

## The gate

A full census takes about 75 minutes with 16 jobs, so it runs one at a time
under a lock. An engine change is measured as a **pair**: the base commit and
the change, built with the same toolchain and Skia flags, sharing one cache of
Chromium captures. The record lists exact counts before and after, and every
comparison gained, lost, improved or made worse.

The rule for engine changes is **no regression**: no exact comparison may be
lost. Counts from different toolchains are not compared with each other.

## Current numbers

| Measurement | Result | Source |
| --- | --- | --- |
| Census, expanded, Linux | **22,517 of 23,728 exact (94.9%)**, 0 errors | [WOFF2 decode record](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/woff2-decode-2026-11-09.md), 9 Nov 2026 |
| CSS Grid coverage | **2,218 of 4,204 exact (52.8%)** | [Grid coverage record](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/grid-coverage-2026-11-09.md), 9 Nov 2026 |
| `oklch()`, `oklab()`, `color-mix()` swatches | **54 of 54 exact**, Linux and Windows oracles | [Color functions record](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/color-functions-2026-11-09.md), 9 Nov 2026 |
| Forced scrollbar frames | **46 of 50 exact** | [Scrollbar frame clip record](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/native-scrollbar-frame-clip-2026-10-20.md), 20 Oct 2026 |
| shadcn/ui separator | **8 of 16 exact** (all light; dark not remeasured since a color fix) | [Components track](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/tracks/components.md), 10 Nov 2026 |

The census figure is from paired runs with the CI Skia build flags in WSL.
It was last raised on 5 November, from 22,516 to 22,517, by the
[spaced pattern record](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/spaced-pattern-extent-2026-11-05.md).
The two engine records since then, for color functions and WOFF2 decoding,
each measured 22,517 before and after, with no Open UI image changed. Each engine record in
[`docs/progress/`](https://github.com/zhuowcui/open-ui/tree/main/docs/progress)
gives its commands, commits and a JSON delta under `evidence/`.

## Known failures

The census has **1,211 comparisons that are not exact**. Beyond those:

- **Grid.** In the coverage suite, 39 tests panic in grid area placement, and
  two float-only references grow memory without bound and are not measured.
  Most other differences are large layout differences.
- **Real-font text.** Real-font evidence is much weaker than the Ahem census.
  Chromium Linux LCD comparisons with real fonts were 817 of 904 exact in the
  latest expanded census, and an installed-font advance bug is open (5.9375px
  where Chromium places 6px). shadcn/ui sets almost everything in Geist or
  Inter at 14px, so this is the main blocker for exact components.
- **Components with text.** After WOFF2 fonts started loading on Linux
  ([#97](https://github.com/zhuowcui/open-ui/pull/97)), the components track
  measured its 192 comparisons on a working branch: 8 exact, 184 different,
  0 errors. Layout matches for buttons, badges and labels; glyph placement,
  badge edges, translucent borders and an opacity layer differ. That is the
  track's measurement, not a CI result.
- **Native scrollbar clipping.** The [17 October checkpoint](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/branch-checkpoint-2026-10-17.md)
  merged with one documented failing test: a native scrollbar frame clip guard
  at 1.25× scale. The [20 October repair](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/native-scrollbar-frame-clip-2026-10-20.md)
  fixed the paint and replaced the guard's pixel-centre rule with Chromium's
  measured extent, raising exact frames from 40 to 46 of 50. Four states
  remain inexact, at fractional box edges that the compositor clips or covers
  differently.
- **Color functions.** `color-mix()` with `currentcolor`, other color spaces,
  `calc()` channels and relative colors are rejected rather than approximated.
  Out-of-gamut gradient stops are clamped where Chromium keeps floats
  (unmeasured), and transitions interpolate in sRGB, not oklab.
- **GPU raster.** Open UI rasterizes on the CPU. The Ganesh GPU path is not
  promoted (408 of 640 focused comparisons exact).

## Not measured

- Text in general, tables and transforms outside the census areas above.
- Startup time, memory and frame time. No benchmark is published until the
  benchmark harness has measured them.
- Windows behavior beyond its own tests: IME, drag and drop, UI Automation
  behavior and packaging.
- macOS, which has no port and no oracle yet.

## Older figures

Earlier records quote other totals, such as 21,659 or 21,663 of 23,728 in
mid-October. Some were measured on private sources or with a different
toolchain, and those records say so. They are history, not the current
number.
