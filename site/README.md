# Project website

The Open UI website: a landing page, docs and a components page, built with
[Astro](https://astro.build/) 7.3.8 and
[Starlight](https://starlight.astro.build/) 0.42.8, with Pagefind search. It
is published to GitHub Pages at <https://zhuowcui.github.io/open-ui/> by
`.github/workflows/site.yml`.

"Open UI" is a working name; this project is unrelated to the W3C Open UI
Community Group.

## Commands

Node 22.18 or later works locally (some dependencies ask for 22.23.4 and
print an engine warning on older 22.x); CI uses Node 24.22.1.

```sh
cd site
npm ci
npm run dev       # http://localhost:4321/open-ui/
npm run build     # static site in dist/
npm run preview   # serve dist/
npm run check-assets
```

`dev` and `build` first run `scripts/sync-assets.mjs`, which copies Geist and
Geist Mono from `components/assets/fonts/geist/` after checking the SHA-256
recorded in `scripts/assets.json`.

## Screenshots

Every app image on the site is drawn by Open UI, headless, and committed in
`src/assets/showcase/` as rendered. The site's build only reads them, so
site CI never builds Rust or Skia.

| Path | What it is |
| --- | --- |
| `showcase/src/main.rs` | The scenes: the dashboard on the home page and the component tiles, light and dark, at device scale 2 |
| `showcase/src/example.rs` | The card in the home page's code example; the page shows this file's source next to its render |
| `showcase/render.ps1` | Builds the renderer with the CI Skia flags and the shared prebuilt Skia, and writes every scene (Windows) |
| `showcase/reference/example.html` | The same card as HTML, with shadcn/ui's markup and compiled CSS from `components/reference/pages` |
| `showcase/capture.py` | Draws that page with the pinned Chromium through the components harness (Linux or WSL) |

```powershell
powershell -File site\showcase\render.ps1            # every scene
powershell -File site\showcase\render.ps1 app team   # some scenes
```

```sh
python3 site/showcase/capture.py   # example-chromium-light.png
```

`showcase` is its own Cargo workspace, like `components/`, and uses only the
public `openui` crate and `openui-components`. Its scenes stack content in
block flow and use flex only for rows: Open UI's column flex boxes still
size text items as if at their min-content width, which is also why the
`Card` component is not used in them yet.

## Content

| Path | Page |
| --- | --- |
| `src/pages/index.astro` | Home: hero, code and result, features, components, the browser comparison, call to action |
| `src/content/docs/docs/` | Introduction, getting started, views and styles, testing, rendering, roadmap, about |
| `src/content/docs/components.mdx` | Components |
| `src/content/docs/blog/` | Blog |
| `src/components/` | `Shot` (a light and dark screenshot), `Compare` (the comparison slider), `CopyCommand`, `Icon` (Lucide), `HeaderLinks` (the docs header's links) |
