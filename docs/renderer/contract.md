# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147; HTML parsing, JavaScript execution,
navigation, networking, iframe browsing contexts, storage, and media playback
are not part of the renderer contract.

The generated v1 contract records four complete-suite profiles and the
40-profile focused viewport/scale cross-product. The 800×600@1 profile refers
to the existing 5,731-case baseline by hash. The generator refuses to rewrite
or accept drift in that legacy baseline.
The Chromium source/API inventory remains pinned to `147.0.7727.24`; Linux
pixel qualification records and verifies the installed `147.0.7727.50`
Chrome-for-Testing raster oracle used by the frozen comparison harness.

Layout, input, hit testing, scrolling, selection, and accessibility use logical
CSS pixels. Paint commands are recorded in logical coordinates and replayed to
the profile's physical surface after applying device scale. Whole-frame
resampling is not a conforming presentation path.

`author-style-inventory.json` classifies every `ComputedStyle` field observed
by the layout and paint source trees. Engine bookkeeping has an explicit
`internal` classification; all remaining fields are author-facing typed
properties. An unclassified field is a qualification failure.

`javascript-disposition.json` re-audits the frozen 1,912-row browser-script
partition. Pure final-state mutation candidates remain candidates until their
native lowering is pixel-qualified. Behavioral rows do not count against
renderer coverage because the product does not embed a browser runtime.

Application and system font ownership, registration limits, and C handle
lifetime rules are documented in [font collections](font-collections.md).
Font selection and shaping precedence is documented in
[font selection and shaping](font-selection-and-shaping.md), and the shared
layout/paint flow is documented in [text layout and paint](text-layout-and-paint.md).

`tools/qualification/run_renderer_matrix.py` is the executable gate for the
four complete profiles and the 40-profile focused cross-product. It verifies
the installed Chromium raster-oracle build, rejects incomplete IDs and missing
templates, checks physical PNG dimensions, compares decoded RGBA pixels with
zero tolerance, and records binary, resource, font, PNG, and pixel hashes. A
partial `--profile` or `--test-id` run is marked incomplete in its report and
cannot be represented as full contract evidence. Use `--plan` to inspect the
case count without producing or mutating qualification evidence.
