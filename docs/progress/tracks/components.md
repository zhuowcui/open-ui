# Components track progress

Workstream C of the [alpha plan](../../alpha/plan.md): shadcn/ui components
for Open UI under the working name `openui-components`, each measured
against shadcn/ui rendered by pinned Chromium 147. The area is
[`components/`](../../../components/README.md). Results are from this
working branch's harness; they are not census results and do not qualify the
renderer.

## Results

| Component | Fixtures | Exact | Evidence | Measured at |
| --- | --- | --- | --- | --- |
| font | Geist, Geist Mono, Inter; light, dark | 0 of 24 (24 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `c98950ef`, 9 November 2026 |
| separator | horizontal, vertical; light, dark | 8 of 16 | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `c98950ef`, 9 November 2026 (unchanged since `7fc2c227`) |

Each fixture is compared at the four census profiles (`legacy-800x600@1`,
`mobile-375x667@2`, `desktop-1280x720@1.25`, `desktop-1920x1080@1.5`).

## 9 November 2026: bundled fonts and icons (C4)

### Change

| Change | Owner | Source |
| --- | --- | --- |
| Bundled assets: Geist and Geist Mono variable WOFF2 (`geist` 1.9.2), Inter variable WOFF2 (`inter-ui` 4.1.1) and the 20 Lucide icons the new-york-v4 components import (`lucide-static` 1.70.0), copied unmodified from the locked npm packages with their OFL and ISC license files. `build.mjs` writes them with `assets/manifest.json` (SHA-256 of each file) and `--check` fails if any differs. | components | `components/assets/`, `components/reference/build.mjs` |
| The reference pages load the fonts with `@font-face` (`font-display: block`, no fallback family) and use Geist as `--font-sans` and Geist Mono as `--font-mono`, as the shadcn/ui site does, plus `--font-inter`. | components | `components/reference/build.mjs` (`FONT_FACES`, `fontCss`) |
| `openui_components::fonts::register_fonts` registers the same files with the same descriptors through `Document::register_font_face`; a test checks the SHA-256 Open UI reports for each registered face against the manifest. | components | `components/openui-components/src/fonts.rs`, `tests/assets_manifest.rs` |
| Font specimen fixtures: three `text-sm` lines at weights 400, 500 and 600 in each face. Fixtures without text skip font registration, so a font-loading failure cannot hide their results. | components | `components/reference/fixtures.tsx`, `components/reference-render/src/main.rs` |
| `reference_harness.py fonts` records the face Chromium draws each specimen line with (`CSS.getPlatformFontsForNode`) and fails on any fallback glyph. | components | `components/harness/reference_harness.py` |

### Measurement

- Chromium draws every specimen line entirely with the bundled face: Geist
  (`Geist-Regular`), Geist Mono (`GeistMono-Regular`) and Inter
  (`InterVariable`), with no fallback glyphs, and each family's `FontFace`
  is loaded
  ([`fonts-v1.json`](../../../components/evidence/fonts-v1.json)).
- Open UI cannot load the fonts in the census real-font environment: all 24
  font comparisons are errors (`Engine(Font(MalformedFont))` from
  `register_font_face`). The separator results are unchanged: 8 exact, and 8
  dark comparisons different by one level on the line pixels.

### Remaining failures

- **WOFF2 fonts do not load on Linux with the census FreeType, 24
  comparisons.** On Linux, `platform_font_data` passes WOFF and WOFF2 bytes
  to FreeType undecoded; FreeType reads WOFF2 only when built with Brotli,
  and the pinned real-font FreeType
  (`tools/accountability/data/fonts/sp16_linux_x86_64/libfreetype.so.6`) is
  not, so registration fails with `MalformedFont`. With the system FreeType
  (which links Brotli) the same render succeeds, and on Windows the bytes are
  decoded with `wuff` first, so the Windows tests pass. Chromium decodes web
  fonts with its OpenType Sanitizer before FreeType on every platform.
  Owner: engine track, text (`bindings/rust/openui-text/src/font/collection.rs`,
  the `#[cfg(not(windows))] platform_font_data`). Not worked around: the
  bundled files stay WOFF2.

Two further text failures were seen in a local preview that is **not part
of the evidence**: Open UI was given the Geist TTF files from the same
`geist` package (which loads with the census FreeType), and only the Geist
fixtures were compared. They are recorded for the engine track and will be
measured once WOFF2 loads.

- **A column flex item with text is as tall as its min-content layout.** In
  the specimen's `display: flex; flex-direction: column; gap: 4px`
  container, each `<p>` is laid out 768px wide but is as tall as one line
  per word (7 words: 140px, where Chromium gives 20px); the text still paints
  on one line at the top of the box. Owner: engine track, layout (flex).
- **Glyphs sit at a different vertical position.** Rebuilt with block layout
  and the same spacing, the first line's ink spans the same columns as
  Chromium's (x 17 to 256 at 1x) but each line covers different rows (at
  1x, Chromium 21–32, 45–56 and
  68–80; Open UI 19–31, 43–55 and 67–79). No whole-pixel shift aligns them,
  so the difference is in baseline or vertical glyph placement, not an
  offset. Owner: engine track, text.

### Framework notes

- `style:line-height="calc(1.25 / 0.875)"` (Tailwind's `text-sm` line
  height) is rejected by `view!`; the fixture uses the number `1.4285714`.
  Owner: engine track (style parsing, RFC 0001).

### Verification

| Check | Command | Result |
| --- | --- | --- |
| Reference pages and assets are current | `node build.mjs --check` in `components/reference`, after `npm ci` | current, 13 pages and 27 assets |
| Harness unit tests | `python3 -m unittest components/harness/test_reference_harness.py` | 14 passed |
| Fonts Chromium uses | `reference_harness.py fonts --output components/evidence/fonts-v1.json` in WSL | 6 pages, 0 problems |
| Comparison | `reference_harness.py compare --renderer <release openui-components-reference> --repository-commit c98950ef… --evidence components/evidence/reference-v1.json` in WSL, renderer built with the CI `SKIA_GN_ARGS` | 8 of 40 exact, 8 different, 24 errors; complete |
| Workspace tests (Windows) | `cargo test --locked` in `components/` | 8 passed |
| Formatting | `cargo fmt --all --check` in `components/` | clean |

## 9 November 2026: reference harness and separator

### Change

| Change | Owner | Source |
| --- | --- | --- |
| Pinned shadcn/ui reference: shadcn `4.23.0` (`96da9cc`, `new-york-v4` on Radix, neutral theme), React 19.3.1, Radix 1.8.1, Tailwind 4.3.5 and tw-animate-css 1.4.0 from a locked npm tree. `build.mjs` renders fixtures to static light and dark pages and compiles shadcn's `globals.css`; `--check` fails if the committed pages are stale. | components | `components/reference/` |
| Harness: captures each page with Chrome-for-Testing 147.0.7727.50 in the census real-font environment, renders the same fixture with Open UI (`chromium-linux-lcd`), compares RGBA bytes and writes evidence. `tokens` records Chromium's sRGB conversion of every theme token. | components | `components/harness/reference_harness.py` |
| Theme stand-ins: the 31 neutral tokens in both themes, as the unclamped float sRGB values Chromium computes, checked against the evidence by a test. To be replaced by typed theme tokens (A2). | components | `components/openui-components/src/theme.rs`, `components/evidence/theme-tokens-v1.json` |
| Separator, the first component, and `openui-components-reference`, which renders each fixture with the public `openui` crate. | components | `components/openui-components/src/separator.rs`, `components/reference-render/` |
| CI: a `components` workflow checks the reference pages, runs the harness unit tests, and formats and tests the components workspace on Linux. | components | `.github/workflows/components.yml` |

Licenses are listed in [`components/THIRD_PARTY.md`](../../../components/THIRD_PARTY.md):
shadcn/ui, Tailwind, Radix, React and tw-animate-css are MIT,
class-variance-authority is Apache-2.0, Lucide is ISC, and Geist and Inter
are OFL 1.1.

### Measurement

| Theme | `legacy-800x600@1` | `mobile-375x667@2` | `desktop-1280x720@1.25` | `desktop-1920x1080@1.5` |
| --- | --- | --- | --- | --- |
| light (2 fixtures) | 2 exact | 2 exact | 2 exact | 2 exact |
| dark (2 fixtures) | 2 different | 2 different | 2 different | 2 different |

Every dark difference is confined to the separator's own pixels (for
example 256 pixels, the 256 × 1 line, at 1x), and each differs by one level:
Chromium paints the line `rgb(34 35 35)` and Open UI `rgb(34 34 34)`.

### Remaining failures

- **Extended-range token colors are clamped before painting, 8
  comparisons.** The dark `--border` is `oklch(1 0 0 / 10%)`, which Chromium
  converts to `color(srgb 0.999871 1.00005 1.00007 / 0.1)` and paints from
  those floats: green is 0.1 × 1.00005 + 0.9 × 10/255 = 0.13530, 34.501 of
  255, which rounds to 35. Open UI clamps each channel to [0, 1] before
  painting, giving 34.4999, which rounds to 34. Owner: engine track, colors
  (A2), `openui-paint/src/painter.rs` (`set_paint_css_color_with_alpha`
  clamps `r`, `g` and `b`). The same clamp will affect any token outside
  sRGB, including `destructive` and the chart colors. The stand-in keeps
  Chromium's floats, so no component code works around it.

### Framework notes

- `view!` does not fill an omitted `#[prop(optional)]` prop: the props
  struct needs every field, and an optional prop must be passed as
  `Some(..)`. `Separator` takes `orientation` as a required prop for now.
- A `{expr}` child in `view!` is always text; a child view can only be
  embedded through a component. The fixture renderer uses a one-line
  `FixtureView` component for this.

These are API requests for the framework track, not blockers.

### Verification

| Check | Command | Result |
| --- | --- | --- |
| Reference pages are current | `node build.mjs --check` in `components/reference`, after `npm ci` | current, 7 files |
| Harness unit tests | `python3 -m unittest components/harness/test_reference_harness.py` | 10 passed |
| Theme tokens | `reference_harness.py tokens --output components/evidence/theme-tokens-v1.json` in WSL | 62 tokens (31 per theme) |
| Comparison | `reference_harness.py compare --renderer <release openui-components-reference> --repository-commit 7fc2c227… --evidence components/evidence/reference-v1.json` in WSL, renderer built with the CI `SKIA_GN_ARGS` | 8 of 16 exact, 8 different, 0 errors; complete |
| Workspace tests (Linux) | `cargo test --release --locked` in `components/`, WSL | 6 passed |
| Formatting | `cargo fmt --all --check` in `components/` | clean |

## Next

1. Static components with inline styles: button (all variants and sizes),
   card, badge and label, measured the same way. Their text cannot match
   until the WOFF2 loading failure is fixed; the fixtures are still built and
   measured so the results update when it is.
2. The Radix-like primitives design (C2) in
   `docs/alpha/rfcs/0003-component-primitives.md`, while waiting for typed
   style rules for the interactive components.
3. Lucide icons in Open UI: the static SVG decoder
   (`bindings/rust/openui-paint/src/image_resource.rs`) draws only `<rect>`
   shapes, while every Lucide icon is stroked `<path>` data in
   `currentColor`. Owner: engine track (paint); requested before the first
   component that shows an icon.
