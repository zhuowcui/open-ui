# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147; HTML parsing, JavaScript execution,
navigation, networking, iframe browsing contexts, storage, and media playback
are not part of the renderer contract.

The generated v1 contract records four complete-suite profiles and the
40-profile focused viewport/scale cross-product. The 800×600@1 profile refers
to the existing 5,731-case baseline by hash. The generator refuses to rewrite
or accept drift in that legacy baseline.

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
