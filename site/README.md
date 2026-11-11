# Project website

The Div website: a landing page, docs and a components page, built with
[Astro](https://astro.build/) 7.3.8 and
[Starlight](https://starlight.astro.build/) 0.42.8, with Pagefind search. It
is published to GitHub Pages at <https://div.build/> (`public/CNAME`) by
`.github/workflows/site.yml`.

The project's name, domain and repository URL are set once, in
`src/lib/project.mjs`. Code samples use the names the code has on `main`
(`openui`, `openui-cli`).

## Commands

Node 22.18 or later works locally (some dependencies ask for 22.23.4 and
print an engine warning on older 22.x); CI uses Node 24.22.1.

```sh
cd site
npm ci
npm run dev       # http://localhost:4321/
npm run build     # static site in dist/
npm run preview   # serve dist/
npm run check-assets
npm run reference         # regenerate src/data/elements.json
npm run check-reference   # fail if elements.json is out of date (CI runs this)
```

`dev` and `build` first run `scripts/sync-assets.mjs`, which copies Geist and
Geist Mono from `components/assets/fonts/geist/` after checking the SHA-256
recorded in `scripts/assets.json`.

## Screenshots

Every app image on the site is drawn by Div, headless, and committed in
`src/assets/showcase/` as rendered. The site's build only reads them, so
site CI never builds Rust or Skia.

| Path | What it is |
| --- | --- |
| `showcase/src/main.rs` | The scenes: the dashboard on the home page and the component tiles, light and dark, at device scale 2; then every docs example |
| `showcase/src/example.rs` | The card in the home page's code example, built from base elements; the page shows this file's source next to its render |
| `showcase/src/docs/` | The docs examples. `Example.astro` shows each file whole next to its render, `docs-<name>-light.png` |
| `showcase/src/snippets/` | Code the docs show that cannot be drawn headless (windows, menus, dialogs, HTTP, tests), and `fragments.rs`, which holds the short inline fragments from the docs pages. Compiled, never run; `Snippet.astro` shows a file whole |
| `showcase/render.ps1` | Builds the renderer (and compiles its tests) with the CI Skia flags and the shared prebuilt Skia, and writes every scene and docs example (Windows) |
| `showcase/reference/example.html` | The same card as HTML, with Geist and a small reset that matches Div's lack of default styles |
| `showcase/capture.py` | Draws that page with the pinned Chromium through the components harness (Linux or WSL) |

```powershell
powershell -File site\showcase\render.ps1            # every scene and docs example
powershell -File site\showcase\render.ps1 app team   # some scenes
powershell -File site\showcase\render.ps1 docs       # only the docs examples
```

So every example on the site compiles against the checked-out `openui` and
is drawn by it. When a page's inline fragment changes, change it in
`showcase/src/snippets/fragments.rs` too.

```sh
python3 site/showcase/capture.py   # example-chromium-light.png
```

`showcase` is its own Cargo workspace, like `components/`, and uses only the
public `openui` crate and `openui-components`. Its scenes stack content in
block flow and use flex only for rows: Div's column flex boxes still
size text items as if at their min-content width, which is also why the
`Card` component is not used in them yet.

## Generated tables

The Elements and Styling reference pages read tables generated from the
Rust source, so they list exactly what `main` supports.

| File | Written by | Checked |
| --- | --- | --- |
| `src/data/elements.json` (tags, kinds, default display, roles, input types) | `npm run reference` (`scripts/gen-reference.mjs`, which reads `openui` and `openui-engine` source) | In CI, by `npm run check-reference` |
| `src/data/properties.json` (258 style properties, their literal grammar, Rust types and setters) | `site/tools/style-table`, which runs `openui-style`'s parser | Locally: rerun it and commit when `openui-style` changes, since site CI builds no Rust |

```sh
cargo build --release --locked --manifest-path site/tools/style-table/Cargo.toml
python3 site/tools/style-table/generate.py
```

`style-table` depends only on `openui-style`, so it builds without Skia.

## Content

| Path | Page |
| --- | --- |
| `src/pages/index.astro` | Home: hero, code and result, features, components, the browser comparison, call to action |
| `src/content/docs/docs/` | Introduction, getting started, rendering, roadmap, about |
| `src/content/docs/docs/concepts/` | Core concepts: views, elements, styling, layout, text and fonts, events and state, apps and windows |
| `src/content/docs/elements/` | The Elements reference: one page per element or group of tags |
| `src/content/docs/reference/` | The Styling reference: properties, values and units, selectors and state |
| `src/content/docs/docs/services/` | App services: windows, menus, dialogs, clipboard, HTTP, async tasks, testing, CLI |
| `src/content/docs/components.mdx` | The component library (secondary, built on the elements) |
| `src/content/docs/blog/` | Blog |
| `src/components/` | `Shot` (a light and dark screenshot), `Compare` (the comparison slider), `CopyCommand`, `Icon` (Lucide), `HeaderLinks` (the docs header's links), `Example` and `Snippet` (docs code with its render, or compiled code alone), `ElementTable` and `PropertyTable` (the generated tables) |
