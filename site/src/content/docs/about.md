---
title: About this site
description: What this site is built with, and what Open UI is not.
---

**This site is HTML.** Open UI renders native windows and does not output
HTML, so the site is built with [Astro](https://astro.build/) and
[Starlight](https://starlight.astro.build/), served by GitHub Pages and drawn
by your browser. Its source is in
[`site/`](https://github.com/zhuowcui/open-ui/tree/main/site).

**Not the W3C Open UI Community Group.** "Open UI" is this project's working
name until it picks its own. It is unrelated to the W3C
[Open UI Community Group](https://open-ui.org/) at open-ui.org, which works on
web platform controls.

**Images.** The comparison on the home page uses committed Chromium captures
and Open UI renders, copied unmodified by
[`site/scripts/sync-assets.mjs`](https://github.com/zhuowcui/open-ui/blob/main/site/scripts/sync-assets.mjs).
The script checks each file's SHA-256 against
[`site/scripts/assets.json`](https://github.com/zhuowcui/open-ui/blob/main/site/scripts/assets.json), counts the
differing pixels itself and fails if a count differs from the record. Your
browser recounts them when you open the page.

**Fonts.** Geist and Geist Mono, under the SIL Open Font License 1.1
([license](/open-ui/fonts/Geist-OFL.txt)), from the files bundled in
`components/assets/fonts/`.

**Status.** Open UI is an experimental alpha. Numbers on this site link to the
committed record they come from.
