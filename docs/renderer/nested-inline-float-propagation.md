# Nested inline floats and flow-root height

The target is pinned Chromium 147. Open UI renders the final state of
`wpt/css2_floats/adjoining-floats-dynamic` through native Engine mutations;
it does not run the test's JavaScript. The mutation IR reads layout, then sets
the target float's width to 50 CSS pixels.

The reduced final state has a 100-pixel-wide red `display: flow-root` box, a
50-by-50 right float, and a cleared nested block containing two 50-by-50
left-floated spans. The cleared block begins 50 pixels below the flow root's
top. Its nested floats therefore extend to 100 pixels. The flow root's
automatic height must include their lower margin edge. Before this change,
the pure inline layout branch kept its float exclusions locally, and the red
background stopped 40 pixels early. At 1280×720@1.25, this exposed 50 green
edge pixels over white where Chromium composites them over red.

`openui-layout::block_layout` now passes only newly created float exclusions
through ordinary ancestor blocks. A new block formatting context uses its
own float exclusions to set its automatic block size. The low-level layout
test `flow_root_contains_floats_from_nested_pure_inline_context` checks the
inherited right float, two nested left floats, and 100-pixel flow-root height.

Clean checkpoint `8950b426` gives this pending case **4/4 exact** against
Chromium. The complete original [v27 census](generated/four-profile-census-v27.json)
remains **21,225/22,924 exact**, with 1,699 differences and zero errors. Every
original residual diff signature matches v26. The clean
[v28 raster index](generated/focused-primitive-raster-v28.json) remains 640/640
focused and 960/960 primitive exact. The complete
[v13 expanded requalification](generated/expanded-requalification-v13.json)
still has 197 of the prior 200 additions exact at all four profiles, with the
same three demotions. The [pending-case index](generated/pending-mutation-candidates-v2.json)
records one newly eligible case out of 36; it is not yet in the admitted
expanded manifest.

The pending `sticky_sticky-continuation-crash` case remains different. Its
Chromium source combines a moved float, sticky positioning, and columns. Its
wrong-pixel counts rose from 1,076 to 1,332; 4,278 to 5,302; 1,668 to 2,068;
and 2,399 to 2,975 across the four required profiles. The additional wrong
region is a black 16-pixel-wide continuation below the prior mismatch. This
case needs its own sticky/fragmentation root cause and cannot be admitted.

The layout package tests pass with a 32 MiB test-thread stack. One existing
multicolumn test overflows the default stack at both this checkpoint and its
parent commit; the change did not introduce that failure. Hosted Rust parity
and conformance checks passed on the PR. The original pixel gate and release
qualification remain open.
