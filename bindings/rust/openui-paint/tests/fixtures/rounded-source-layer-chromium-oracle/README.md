# Promoted rounded primitive Chromium oracle

These five full RGBA frames are decoded from two independent stock Chromium147.0.7727.50 captures per scale of the immutable32×32 input with one blue20×20 box,3px corner radii and `will-change:transform`. Effective physical dimensions are32,40,48,56 and64 square pixels; the browser starts with a visible800×600 window before applying that exact viewport. Raw HTML, PNG, RGBA, DOM, native dump and process receipts are pinned in `provenance.json`.

No pixels were adjusted. Native N32 readback isBGRA8888 on the qualification platform; the guard swaps red/blue channels without modifying any channel values and compares every pixel with zero tolerance. The original helper is unchanged. Its direct canonical/exact-spine raster differs from the actual Chromium oracle by10pixels at1.75 and2.0, while local-source replay matches allfive. Other modes and host-state guards retain their existing assertions.

This corrects a test oracle for one measured primitive. Production rendering, original fixture identity, independently selected layers, final Viz execution and release qualification remain unchanged/unqualified.
