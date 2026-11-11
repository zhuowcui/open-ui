# Project website

The Open UI website: a landing page and docs built with
[Astro](https://astro.build/) 7.3.8 and [Starlight](https://starlight.astro.build/)
0.42.8, with Pagefind search. It is published to GitHub Pages at
<https://zhuowcui.github.io/open-ui/> by `.github/workflows/site.yml`.

"Open UI" is a working name, and the site says so; this project is unrelated
to the W3C Open UI Community Group.

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

`dev` and `build` first run `scripts/sync-assets.mjs`.

## Assets

The site copies, never edits, files committed elsewhere in the repository:

- Geist and Geist Mono from `components/assets/fonts/geist/` (SIL OFL 1.1).
- The comparison images on the home page: Chromium 147.0.7727.50 captures and
  Open UI renders from `docs/renderer/evidence/native-fragmented-fieldset-*`.

[`scripts/assets.json`](scripts/assets.json) lists every source path, its
SHA-256 and, for each image pair, the differing-pixel count its record
states. `scripts/sync-assets.mjs` checks the digests, decodes each PNG pair,
counts the pixels whose RGBA bytes differ and fails if a count differs from
the record. It writes the copies and `src/assets/generated/comparisons.json`;
all of these are ignored by git. The home page recounts the pixels in the
visitor's browser when it draws the diff overlay.

To use another pair, add it to `assets.json` with its digests and recorded
count, and add its directory to the workflow's `sparse-checkout` list.

## Content

| Path | Page |
| --- | --- |
| `src/pages/index.astro` | Home, with `src/components/CompareSlider.astro` |
| `src/content/docs/getting-started.md` | Getting started, from the CLI and template on `main` |
| `src/content/docs/fidelity.md` | How measurement works, current numbers, known failures |
| `src/content/docs/{docs,devlog}/index.md`, `components.md`, `roadmap.md`, `about.md` | Short stubs |

Every number on the site cites the committed record it comes from. When a
record changes, update the page that quotes it.
