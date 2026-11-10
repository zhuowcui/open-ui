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
| separator | horizontal, vertical; light, dark | 8 of 16 | [`reference-v1.json`](../../../components/evidence/reference-v1.json) | `7fc2c227`, 9 November 2026 |

Each fixture is compared at the four census profiles (`legacy-800x600@1`,
`mobile-375x667@2`, `desktop-1280x720@1.25`, `desktop-1920x1080@1.5`).

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

1. C4 assets: bundle Geist, Inter and Lucide icons with their licenses, load
   them in the reference pages with `@font-face`, and register the same files
   in Open UI.
2. Static components with inline styles: button (all variants and sizes),
   card, badge and label, measured the same way.
3. The Radix-like primitives design (C2) in
   `docs/alpha/rfcs/0003-component-primitives.md`, while waiting for typed
   style rules for the interactive components.
