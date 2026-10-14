# Adjacent row flex items across multicolumn slices

Chromium 147 is the pixel target. The historical Open UI image archive is
provenance, not an expected image. This repair changes multicolumn layout
fragments before painting; it does not alter the Chromium captures, reference
images, raster backend, or pixel comparator.

The existing WPT `flexbox_multi-line-row-flex-fragmentation-001` is a reduced,
Engine-backed reproducer: four opaque, fixed-height 250 CSS px flex items sit
in a 100 CSS px high, five-column container. In the third column, Open UI
extended the first item's visual fragment from 250 to 300 CSS px even though
its adjacent item began at 250 and covered the rest of the slice. At 1.25×,
that extra paint changed the shared fractional edge. At physical pixel
`(87,88)`, the old Open UI result was RGB `(15,120,0)` while Chromium was
`(63,96,0)`.

`layout_multicol` now records opaque in-flow leaf item bounds before it mutates
the slice. It keeps an authored fixed-height item at its own border-box height
when the adjacent opaque item starts exactly at its bottom, covers its inline
span, and fills the remaining slice. Forced continuations without that neighbor
retain the existing extension. Nested multicolumn contexts retain their own
continuation handling because local neighbor bounds do not describe inner
paint. The targeted layout test checks both the adjacent-item case and the
forced-continuation guard.

| Case | Profile | Previous wrong pixels | Clean checkpoint wrong pixels |
|---|---|---:|---:|
| `flexbox_multi-line-row-flex-fragmentation-001` | 1280×720@1.25 | 87 | 0 |
| `flexbox_multi-line-row-flex-fragmentation-004` | 1280×720@1.25 | 87 | 0 |
| `flexbox_multi-line-row-flex-fragmentation-051` | 1280×720@1.25 | 132 | 14 |
| `flexbox_multi-line-row-flex-fragmentation-051` | 1920×1080@1.5 | 75 | 0 |

A broader diagnostic also changed nested case `-059` at 1.25× from 856 to 862
wrong pixels. It was rejected. The final nested-context guard leaves that
image and its 856 wrong pixels unchanged. Every 1× and mobile 2× image is
unchanged. The remaining 14 pixels in `-051` lie on physical x=87, mostly
y=25–37; their raster-edge cause is still unreviewed and they remain a failing
comparison.

A later dirty diagnostic allowed the same neighbor rule to consider an
authored `break-inside: avoid` item. Across all 125 row-flex fragmentation
cases at four profiles, it changed no comparison status, wrong-pixel count, or
decoded Open UI image, including the neighboring `-021` edge. The trial was
reverted; its report SHA-256 is
`2430815cbb1063246a013e7a7ac47eae7d916498a51fafa5631523bda5744448`.

The clean [v37 complete census](generated/four-profile-census-v37.json) at
`dc451061` is 21,248/22,924 exact, 1,676 different, zero errors. Compared
with v36, only the four Open UI images in the table changed; three became
exact, and no previously exact image regressed. All 22,924 Chromium oracle
identities and decoded images stayed fixed. The census still has 924 unowned
residual test IDs and is diagnostic, not qualifying. The clean
[v38 focused/primitive index](generated/focused-primitive-raster-v38.json) is
640/640 and 960/960 exact, with every Open UI and Chromium image unchanged
from the prior raster v37 index. The clean
[v20 expanded requalification](generated/expanded-requalification-v20.json)
retains 198 of 201 added native final-state cases exact at all four profiles;
all 22,924 original results match the v37 census.

Investigation owner for the fixed cause: multicolumn row-flex continuation in
`openui-layout`. The remaining `-051` edge and `-059` nested fragmentation
differences need separate reviewed causes and remain unowned in the release
census. The full exact renderer gate remains open.
