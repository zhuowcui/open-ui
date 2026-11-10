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
| `openui-components/` | The components, with inline styles and sRGB stand-ins for the theme tokens (`src/theme.rs`). |
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
changes, then update `openui-components/src/theme.rs`; a test checks it):

```bash
python3 components/harness/reference_harness.py tokens \
  --output components/evidence/theme-tokens-v1.json
```

`--fixture` and `--profile` select a subset; such a run is marked
`"complete": false` in its evidence. Images and diffs are written under
`out/components-reference/`. Captures are small: one browser per comparison,
four at a time by default. Do not run them during a full census.

## Stand-ins

Until the engine parses `oklch()` and custom properties, each theme token is
the sRGB color Chromium computes for it, rounded to 8 bits per channel, with
its alpha kept exact (`evidence/theme-tokens-v1.json`). For out-of-gamut
tokens (`destructive` and the chart colors) the stand-in clamps each
channel; whether that is what Chromium paints is known only once a fixture
using the token is measured. These are marked STAND-IN in the source and are
to be replaced by typed theme tokens.
