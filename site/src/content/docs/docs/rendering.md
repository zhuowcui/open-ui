---
title: Rendering
description: How Div draws your views, and how it is checked against a browser.
---

Div has its own layout and paint engine, written in Rust and drawing
with Skia. It follows the same CSS rules a browser does, so that a view
styled with CSS looks the way it would on the web, without shipping a
browser. Where it does not yet, the differences are listed below.

## How it is checked

Chromium is the reference. The test suites render the same page twice, once
in a pinned build of Chromium and once in Div, and compare the two
images pixel by pixel. A change to the engine is accepted only if no test
that matched before stops matching. The suites are drawn from the Web
Platform Tests (block layout, flexbox, grid, positioning, backgrounds,
overflow, fragmentation and more) and from shadcn/ui's components rendered
both ways.

The home page shows one of these pairs: the card from the code example,
drawn by Chromium from HTML and by Div from Rust.

## Where it still differs

- **Text.** Div draws text with the same fonts and shaping as the
  browser, but glyphs can still sit a pixel higher or lower, and some
  antialiasing differs.
- **Column flexbox.** A column flex container can make text items taller
  than a browser would. Until this is fixed, the components that use it
  (such as `Card`) are taller than shadcn/ui's.
- **Grid.** Many grid layouts match, but not all yet.

The measured results, test by test, are in the repository under
[`docs/progress`](https://github.com/zhuowcui/open-ui/tree/main/docs/progress)
for anyone who wants the detail.
