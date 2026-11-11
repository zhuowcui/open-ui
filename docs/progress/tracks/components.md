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
| badge | 6 variants; light, dark | 0 of 48 (48 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 |
| button | 6 variants, sizes xs, sm and lg, disabled; light, dark | 0 of 80 (80 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 |
| card | basic, with action; light, dark | 0 of 16 (16 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 |
| font | Geist, Geist Mono, Inter; light, dark | 0 of 24 (24 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 (unchanged since `c98950ef`) |
| label | basic; light, dark | 0 of 8 (8 errors: fonts do not load) | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 |
| separator | horizontal, vertical; light, dark | 8 of 16 | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `183f3d1b`, 10 November 2026 (unchanged since `7fc2c227`) |

Each fixture is compared at the four census profiles (`legacy-800x600@1`,
`mobile-375x667@2`, `desktop-1280x720@1.25`, `desktop-1920x1080@1.5`).

## 9 November 2026: button, badge, card and label

### Change

| Change | Owner | Source |
| --- | --- | --- |
| Button (variants `default`, `destructive`, `outline`, `secondary`, `ghost`, `link`; sizes `default`, `xs`, `sm`, `lg`; disabled), badge (the same six variants), card (`Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`) and label, over the public `openui` crate. The icon sizes are left out until Open UI can draw Lucide icons; hover, focus and `aria-invalid` styles wait for the interactive work. | components | `components/openui-components/src/{button,badge,card,label}.rs` |
| Styles are the computed values of Tailwind's compiled classes, set declaration by declaration after preflight's reset. Where Open UI's literals cannot express a value the typed value is set (`box-shadow`, `width: fit-content`, `container-type`, `container-name`, grid placement), `border` is set as longhands, and `line-height` lengths stand for Tailwind's `calc()` ratios. `Theme::dark` stands in for the `.dark` class, and `css::with_opacity` for `color-mix(in oklab, X N%, transparent)`. | components | `components/openui-components/src/css.rs`, `src/theme.rs` |
| 19 fixtures (38 pages): each button variant at the default size, the three text sizes, disabled, each badge variant, the docs example card without and with an action, and a label. Buttons and badges sit in a `flex` row. | components | `components/reference/fixtures.tsx`, `components/reference-render/src/main.rs` |
| Geometry diagnostic: the renderer's `geometry` prints each fixture element's border box, and `reference_harness.py geometry` compares it with Chromium's `getBoundingClientRect` (same within 1/128 px), to tell layout from paint differences. It writes no evidence. | components | `components/reference-render/src/main.rs`, `components/harness/reference_harness.py` |

### Measurement

In the census real-font environment, every new fixture is an error: each
draws text, and the bundled WOFF2 fonts do not load there (the C4
failure below). The complete run is 8 of 192 comparisons exact (the light
separators), 8 different (the dark separators, unchanged) and 176 errors
(`Engine(Font(MalformedFont))`) ([`reference-v1.json`](../../../components/evidence/reference-v1.json)).
It was measured again at `183f3d1b`, after the rebase onto main
(`806356cb`); every row is the same as at `df1c5b5d`, the commit before the
rebase, which has the same `components/` tree. The previews below ran at
`df1c5b5d`.

### Preview findings

These are **not part of the evidence**: Open UI was run with the system
FreeType (which reads WOFF2) instead of the census one, so the text loads.
They locate the failures that the font error hides, for the engine track.

- **Layout matches for every button, badge, label and separator fixture.**
  The geometry diagnostic finds each element's border box within 1/128 px
  of Chromium's at all four profiles (152 of 192 fixture layouts; the other
  40 are the font and card fixtures below).
- **Plain rounded rectangles paint exactly; text does not.** For each
  `default`, `secondary`, `ghost` and `link` button and every size, all
  differing pixels at 800x600 are inside the button, away from its edges:
  the glyphs (the C4 vertical placement finding).
- **Three edge differences** remain at 800x600 (pixels within 3px of the
  component's edge):
  - Filled badges (a background, a transparent 1px border,
    `overflow: hidden`, a radius of half the height): 29 to 78 pixels on
    the rounded ends. An exact 11px radius gives the same pixels as the
    `9999px` stand-in for `calc(infinity * 1px)`, so the stand-in is not
    the cause; the outline badge, with a border and no background, has
    exact edges. Owner: engine track, paint.
  - Translucent dark borders: the dark outline button (`--input`, 15%) has
    228 differing edge pixels and the dark outline badge (`--border`, 10%)
    183. Both are drawn in the translucent, slightly out-of-gamut white
    tokens whose clamp the dark separator measures, so they are expected to
    change with the A2 color fix; not yet confirmed. Owner: engine track,
    A2 colors.
  - The disabled button (`opacity: 0.5`): 17 and 35 pixels on its rounded
    corners, which are exact on the same button without the opacity.
    Owner: engine track, paint (opacity layers).
- **Column flex items are as tall as their min-content layout.** A
  `display: flex; flex-direction: column` box 382px wide holding one `div`
  with the words "Card Content" lays the `div` out 48px tall (two lines);
  Chromium gives 24px. Every card section is a column flex item, so the
  card is 274px tall (290px with an action) where Chromium's is 190px.
  This is the column flex finding from C4. Owner: engine track, layout
  (flex).
- **A grid `auto` column beside a `1fr` column is sized at min-content.**
  Outside any flex container, `CardHeader` with an action
  (`grid-template-columns: 1fr auto`, the action at `grid-column: 2;
  grid-row: 1 / span 2`) makes the `auto` column 46.73px wide, so "Card
  Action" wraps onto two lines, and the grid 76px tall; Chromium's column
  is the max-content width, 85.61px, and the grid 44px. Owner: engine
  track, layout (grid).

### Framework notes

Open UI's style literals (`view!` `style:` and `typed_style::parse_literal`)
cannot express several values the components need. Each is set another way
and none changes the computed value:

- No `box-shadow` literal: the shadow is built as `Vec<BoxShadow>`.
- `border` needs a width, a style and a color, and rejects
  `currentcolor`; `border-*-width` takes a bare integer, not `1px`.
- No intrinsic lengths (`fit-content`), `container-type` and
  `container-name`, grid placement (`grid-column: 2`) or CSS-wide
  keywords (`inherit`) as literals; no `user-select` property.
- A button must opt out of native appearance
  (`Element::set_native_appearance(false)`); Chromium drops it once a page
  styles the button's background or border.

Owner: engine track (style parsing, RFC 0001) and framework track.

### Verification

| Check | Command | Result |
| --- | --- | --- |
| Reference pages and assets are current | `node build.mjs --check` in `components/reference`, after `npm ci` | current, 51 pages and 27 assets |
| Harness unit tests | `python3 -m unittest components/harness/test_reference_harness.py` | 18 passed |
| Comparison | `reference_harness.py compare --renderer <release openui-components-reference> --repository-commit 183f3d1b… --evidence components/evidence/reference-v1.json` in WSL, renderer built with the CI `SKIA_GN_ARGS` | 8 of 192 exact, 8 different, 176 errors; complete |
| Renderer reproduces | `cargo clean --release -p openui-components-reference -p openui-components`, rebuild at `183f3d1b`, `sha256sum` | `d82152c3…`, the SHA-256 in the evidence |
| Geometry (preview) | `reference_harness.py geometry --renderer <wrapper running the release renderer with the system FreeType>` in WSL | 152 of 192 fixture layouts the same; differences only in the card and font fixtures |
| Workspace tests (Windows) | `cargo test --locked` in `components/`, at `183f3d1b` | 10 passed |
| Formatting | `cargo fmt --all --check` in `components/`, in WSL (on Windows the file list exceeds the command-line limit, os error 206) | clean |

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

`c98950ef` is `2cc95b7a` before its rebases onto main; the two have the same
`components/` tree.

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

1. Remeasure once engine A2 (colors) and the WOFF2 fix merge, and replace
   the stand-ins with real `oklch()` and `color-mix()` values.
2. Build the Radix-like primitives (C2) as proposed in
   [RFC 0003](../../alpha/rfcs/0003-component-primitives.md), once the
   framework and engine tracks have reviewed its requests, then tabs,
   dialog, checkbox and switch on them. Their `data-state` styles need
   typed style rules (A1).
3. Lucide icons in Open UI: the static SVG decoder
   (`bindings/rust/openui-paint/src/image_resource.rs`) draws only `<rect>`
   shapes, while every Lucide icon is stroked `<path>` data in
   `currentColor`. Owner: engine track (paint); requested before the first
   component that shows an icon.
