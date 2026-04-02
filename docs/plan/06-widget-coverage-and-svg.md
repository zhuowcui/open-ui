# Sub-Project 6: Widget Coverage & SVG

> Support all HTML rendering elements and the full SVG stack, with pixel-perfect verification against Chromium.

## Objective

With the full pipeline working (SP5), systematically enable and verify every HTML element type and CSS feature that affects visual rendering. Each element must render identically to Chromium. This includes the complete SVG stack.

**Scope:** Every HTML element listed on MDN that produces visual output, except: `<iframe>`, `<script>`, `<noscript>`, `<object>`, `<embed>`, `<applet>`, `<frame>`, `<frameset>`. Also excludes interactive behaviors that require JavaScript (e.g., `<details>` expand/collapse) — those come from application code.

## HTML Element Coverage

### Block Elements
`<div>`, `<section>`, `<article>`, `<aside>`, `<header>`, `<footer>`, `<nav>`, `<main>`, `<figure>`, `<figcaption>`, `<address>`, `<blockquote>`, `<pre>`, `<hr>`

### Inline Elements
`<span>`, `<a>` (visual only), `<strong>`, `<em>`, `<b>`, `<i>`, `<u>`, `<s>`, `<small>`, `<sub>`, `<sup>`, `<mark>`, `<code>`, `<kbd>`, `<var>`, `<samp>`, `<abbr>`, `<time>`, `<br>`, `<wbr>`

### Headings & Text
`<h1>`–`<h6>`, `<p>`, `<ul>`, `<ol>`, `<li>`, `<dl>`, `<dt>`, `<dd>`

### Table
`<table>`, `<thead>`, `<tbody>`, `<tfoot>`, `<tr>`, `<th>`, `<td>`, `<caption>`, `<colgroup>`, `<col>`

### Form Controls
`<form>`, `<input>` (text, password, checkbox, radio, submit, button, range, color, date, number), `<textarea>`, `<select>`, `<option>`, `<button>`, `<label>`, `<fieldset>`, `<legend>`, `<progress>`, `<meter>`, `<output>`

### Media / Replaced
`<img>`, `<picture>`, `<canvas>` (2D context), `<video>` (poster frame only — no playback)

### SVG (Full Stack)
`<svg>`, `<g>`, `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polyline>`, `<polygon>`, `<path>`, `<text>`, `<tspan>`, `<textPath>`, `<image>`, `<use>`, `<defs>`, `<symbol>`, `<clipPath>`, `<mask>`, `<pattern>`, `<linearGradient>`, `<radialGradient>`, `<stop>`, `<filter>`, `<feGaussianBlur>`, `<feColorMatrix>`, `<feComposite>`, `<feOffset>`, `<feMerge>`, `<feMergeNode>`, `<feFlood>`, `<animate>`, `<animateTransform>`, `<animateMotion>`, `<marker>`, `<foreignObject>`

## CSS Feature Coverage

All CSS properties that affect visual rendering, including:
- Box model (margin, padding, border, width, height, box-sizing)
- Flexbox (all properties)
- Grid (all properties)
- Positioning (static, relative, absolute, fixed, sticky)
- Typography (font-*, text-*, line-height, letter-spacing, word-spacing, white-space)
- Colors & backgrounds (background-*, gradient, color)
- Borders & outlines (border-*, border-radius, outline-*)
- Shadows (box-shadow, text-shadow)
- Transforms (2D and 3D)
- Opacity, visibility
- Overflow, clip, clip-path
- Filters (filter, backdrop-filter)
- Blend modes (mix-blend-mode, background-blend-mode)
- Masks (mask-*)
- Columns (multi-column layout)
- Counters & list styling (list-style-*)
- Table layout properties
- Writing modes (writing-mode, direction, text-orientation)
- Scroll snap (scroll-snap-type, scroll-snap-align)

## Tasks

### Phase A: Block & Inline Elements
1. Enable all block elements (div, section, etc.) with correct default styles
2. Enable all inline elements with correct rendering
3. Headings, paragraphs, lists with correct typography defaults

### Phase B: Table Layout
4. Table rendering with Blink's table layout algorithm
5. Border-collapse, cell spanning, caption positioning

### Phase C: Form Controls
6. Native-rendered form controls using Blink's form element painting
7. Input types: text, checkbox, radio, range, select, button
8. Focus/hover/active visual states

### Phase D: SVG Stack
9. SVG layout integration (SVG coordinate system, viewBox, preserveAspectRatio)
10. SVG basic shapes (rect, circle, ellipse, line, polygon, path)
11. SVG text and text paths
12. SVG gradients, patterns, clip paths, masks
13. SVG filters (Gaussian blur, color matrix, compositing)
14. SVG animations (SMIL)

### Phase E: Advanced CSS
15. CSS filters and backdrop-filter
16. CSS masks and clip-path
17. CSS blend modes
18. Multi-column layout
19. Writing modes (vertical text, RTL)

### Phase F: Pixel-Perfect Verification
20. Build automated pixel-comparison test harness
21. Render each element/feature in both Chromium and Open UI
22. Compare outputs — target < 0.1% pixel difference
23. Build regression test suite (runs on every commit)

## Deliverables

| Deliverable | Description |
|---|---|
| Full HTML element support | All listed elements render correctly |
| Full SVG support | Complete SVG rendering stack |
| `examples/widget_gallery.c` | Gallery showing all supported elements |
| `examples/svg_demo.c` | SVG rendering demo |
| `tests/pixel/` | Automated pixel-comparison test suite |
| Pixel comparison report | Per-element accuracy vs Chromium |

## Success Criteria

- [ ] Every listed HTML element renders with < 0.1% pixel difference vs Chromium
- [ ] SVG basic shapes, text, gradients, filters render correctly
- [ ] Form controls render with correct native appearance
- [ ] Table layout handles colspan/rowspan/border-collapse
- [ ] RTL text and vertical writing modes render correctly
- [ ] Pixel comparison test suite has >95% pass rate
