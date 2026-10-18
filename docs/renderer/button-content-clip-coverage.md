# Native button content clip coverage

Chromium 147 is the pixel oracle. This repair changes the shared paint path
for native button content clipping. It does not change a WPT fixture, Chromium
capture, or finished image.

## Reduced mismatch and cause

The final state of `containing-block-change-button` is a purple button at CSS
x=70 with `position: relative` and `overflow: clip`, containing a green
absolutely positioned box whose left edge is also x=70. At 1.25× both edges
begin at physical x=87.5. The original `-ref` case has the same final visual
state. Each case differed on physical column x=87 for y=275–399: 125 pixels.
Chromium's edge was RGBA `(95, 127, 95, 255)`, while Open UI produced
`(143, 127, 143, 255)`.

The purple button background covers half of that physical column. The green
child also covers half. Chromium applies the child's half-pixel coverage over
the already painted background, giving `(95, 127, 95, 255)`. Open UI had
antialiased the button's implicit content clip and the child's own edge, so
the green coverage was multiplied twice. The renderer now uses a physical-cell
scissor for a `button` with native Button role and two `overflow: clip` axes;
the child's paint still supplies its analytic edge. An authored clip on an
ordinary box retains its existing fractional coverage. The Rust raster test
paints the reduced button/child pair and checks both the partial and full
green cells.

The native final-state fixture reaches the visual state through Engine
mutations. Open UI does not execute the Chromium test's JavaScript. Native
applications use public Rust element operations over the same engine.

## Clean qualification

At clean commit `ee0b04dd`, the rebuilt CPU Skia `pixel_compare` binary had
SHA-256
`77ca77c9fce2374573d9d137712fbfd5673c42bae4ac12d429dee90fac014ff9`.
The complete [v40 original census](generated/four-profile-census-v40.json)
is **21,265/22,924 exact**, 1,659 different, and zero errors. Relative to
v39, only the 1.25× `containing-block-change-button-ref` Open UI image changed;
its 125 wrong pixels became exact. The other 22,923 Open UI decoded images,
all 22,924 Chromium oracle identities, and all Chromium decoded images stayed
fixed. No exact comparison regressed. The raw report is
`out/renderer-evidence/button-clip-full-ee0b04dd/full-summary.json` with
SHA-256
`d3e00513c7d69589e70e9ddd98500dcec023e5590a1b55fdb0c1b1202490698a`.

The [v41 focused/primitive index](generated/focused-primitive-raster-v41.json)
is 640/640 and 960/960 exact across the 40-profile matrices. The complete
[v23 expanded requalification](generated/expanded-requalification-v23.json)
is **22,068/23,728 exact**, 1,660 different, and zero errors. Exactly two
Open UI decoded images changed from v22: the original `-ref` and the native
final-state button case, both at 1.25×; both became exact. All 23,728 Chromium
oracle identities and decoded images stayed fixed, and every original result
matches the separate v40 census. The raw expanded report is
`out/renderer-evidence/button-clip-expanded-ee0b04dd/expanded-summary.json`
with SHA-256
`5917c0e01bcf576a3fc132f040eb923c67e61cf08391cb60b07f648d8463f95f`.

The complete release manifest still includes all 201 additions. The
[v25 diagnostic selection](../../tools/qualification/manifests/expanded-v25.json)
lists the 200 additions exact at all four profiles. The remaining admitted
`display-contents-dynamic-fieldset-legend-001` case differs by 1,315 pixels
at 1.25×. The original census retains 915 unowned residual test IDs. Neither
gate is qualified as complete.
