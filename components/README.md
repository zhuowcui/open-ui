# Components track

shadcn/ui components for Open UI, under the working name
`openui-components`, and the harness that measures each one against shadcn/ui
rendered by Chromium. This area consumes only the public `openui` crate. It
does not change engine or framework crates; requests for them go through an
RFC under `docs/alpha/rfcs/`. Measured results are in
[docs/progress/tracks/components.md](../docs/progress/tracks/components.md).

| Path | What it is |
| --- | --- |
| `Cargo.toml` | A separate Cargo workspace over `bindings/rust/openui`, repeating that workspace's patches. |
| `openui-components/` | The components (separator, button, badge, card, label), with inline styles (`src/css.rs`) and sRGB stand-ins for the theme tokens (`src/theme.rs`), and the bundled fonts (`src/fonts.rs`). |
| `assets/` | Geist, Geist Mono, Inter and Lucide icons, copied unmodified from the locked npm packages by `reference/build.mjs`, with their licenses and `manifest.json`. |
| `reference-render/` | `openui-components-reference`, which renders each fixture with Open UI. |
| `reference/` | Pinned shadcn/ui sources and the Node build of the static reference pages. |
| `reference/pages/` | The generated pages, committed so captures need no Node. |
| `harness/` | Chromium capture, comparison and evidence (`reference_harness.py`). |
| `evidence/` | Committed measurements: theme tokens and comparison results. |
| `THIRD_PARTY.md` | Licenses of everything vendored or compiled in. |

## How a comparison works

1. `reference/build.mjs` renders each fixture in `reference/fixtures.tsx`
   with React's static renderer from the vendored shadcn sources, in a light
   and a dark page. Tailwind 4 compiles shadcn's `globals.css` into
   `pages/reference.css`, scanning only the generated pages. Every input is
   pinned by `package-lock.json` and `reference/shadcn/SOURCE.md`.
2. `reference-render` builds the same fixture IDs with `view!` and the
   components in `openui-components`.
3. `harness/reference_harness.py compare` captures each page with the pinned
   Chrome-for-Testing 147 at the four census profiles, renders the Open UI
   fixture at the same profile, and compares the RGBA bytes. Chromium uses
   the census real-font environment; Open UI uses the `chromium-linux-lcd`
   raster configuration with the pinned FreeType.
4. The results go to `evidence/reference-v<N>.json`: one row per fixture,
   theme and profile, exact or the number of differing pixels, with the
   image hashes and the Chromium, reference and renderer identities.

A fixture ID is `<component>-<state>-<theme>`. The two sides must list the
same IDs; tests on both sides check this against `reference/pages/manifest.json`.

## Commands

Rebuild or check the reference pages (Node 22, any platform):

```bash
cd components/reference && npm ci && node build.mjs --check
```

Build the renderer in WSL with the CI Skia flags (the census build
environment), then compare:

```bash
export FORCE_SKIA_BUILD=1 SKIA_NINJA_COMMAND=ninja CC=clang CXX=clang++
export SKIA_GN_ARGS='extra_cflags += [ "-DSK_SUPPORT_LEGACY_CONIC_CHOP", "-DSK_ENABLE_LEGACY_SHADERCONTEXT", "-DSK_USE_FREETYPE_EMBOLDEN", "-DSK_GAMMA_EXPONENT=1.2", "-DSK_GAMMA_CONTRAST=0.2", "-USK_GAMMA_APPLY_TO_A8", "-fno-strict-aliasing", "-msse3" ]'
cargo build --release --locked --manifest-path components/Cargo.toml -p openui-components-reference
python3 components/harness/reference_harness.py compare \
  --renderer "$CARGO_TARGET_DIR/release/openui-components-reference" \
  --evidence components/evidence/reference-v1.json
```

Record Chromium's sRGB values of the theme tokens (rerun when the theme
changes; a test checks `openui-components/src/theme.rs` against it):

```bash
python3 components/harness/reference_harness.py tokens \
  --output components/evidence/theme-tokens-v1.json
```

Record which faces Chromium draws the font fixtures with; it fails if any
line uses a fallback instead of the bundled web font:

```bash
python3 components/harness/reference_harness.py fonts \
  --output components/evidence/fonts-v1.json
```

Compare each fixture element's border box with Chromium's
`getBoundingClientRect`, to tell layout differences from paint
differences. It is a diagnostic: it writes no evidence unless `--output` is
given, and boxes count as the same within half a layout unit (1/128 px):

```bash
python3 components/harness/reference_harness.py geometry \
  --renderer "$CARGO_TARGET_DIR/release/openui-components-reference"
```

`--fixture` and `--profile` select a subset; such a run is marked
`"complete": false` in its evidence. Images and diffs are written under
`out/components-reference/`. Captures are small: one browser per comparison,
four at a time by default. Do not run them during a full census.

## Stand-ins

Each theme token is the value shadcn's `globals.css` gives it, an
`oklch()` color (`Token` in `openui-components/src/theme.rs`), and Open UI
resolves it as Chromium 147 does; a test checks each one against the sRGB
color Chromium computes (`evidence/theme-tokens-v1.json`). Until the engine
has custom properties, components write the token's value where shadcn
writes `var(--token)`: Tailwind's opacity modifier (`bg-input/30`) is
`color-mix(in oklab, <token> 30%, transparent)` (`css::with_opacity`), and
`Theme::dark` stands in for the `.dark` class that switches Tailwind's
`dark:` variants. These are marked STAND-IN in the source and are to be
replaced by typed theme tokens.

## Styles

Components set the computed values of Tailwind's compiled classes
(`reference/pages/reference.css`) declaration by declaration, after
preflight's reset (`box-sizing: border-box; margin: 0; padding: 0;
border: 0 solid`); see `openui-components/src/css.rs`. Where Open UI's
style literals cannot express a value, the typed value is set instead:
`box-shadow`, `width: fit-content`, `container-type`, `container-name`
and grid placement. `border` is set as longhands, and `line-height`
lengths stand for Tailwind's `calc()` ratios (`text-sm` is 14px over
20px).

## Fonts and icons

The reference pages use Geist as `--font-sans` and Geist Mono as
`--font-mono`, as the shadcn/ui site does, plus Inter as `--font-inter`.
The `@font-face` rules come from `FONT_FACES` in `reference/build.mjs`;
`openui_components::fonts::register_fonts` registers the same files with the
same descriptors, and a test checks both against `assets/manifest.json`.
Lucide icons are bundled for the components that use them, but Open UI's
static SVG decoder draws only `<rect>` shapes so far, so no icon is rendered
yet.
