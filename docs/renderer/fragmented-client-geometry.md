# Fragmented client geometry and physical paint

Chromium 147.0.7727.50 reports source-box client rectangles separately from
physical paint-fragment transforms. These operations can produce different
coordinates for the same element. A client-rectangle center can therefore hit
another element. Open UI must match each measured operation independently.

## Reduced input and measured result

This input needs no script or font:

```html
<!doctype html>
<style>
html,body { margin:0 }
div { display:block; border:0; padding:0; margin:0 }
#columns { width:200px; height:40px; column-count:2;
           column-gap:0; column-fill:auto }
#outer { width:100px; height:100px; transform:scale(.5) }
#child { position:fixed; top:0; left:0; width:8px;
         height:8px; background:green }
</style>
<div id="columns"><div id="outer"><div id="child"></div></div></div>
```

With a 400×240 CSS-pixel viewport, the unchanged pinned Chromium capture gives:

| Operation | Measured result in CSS pixels |
| --- | --- |
| Outer client rectangles | (25,25,50,20), (75,25,50,20), (125,25,50,10) |
| Child client rectangle | (25,25,4,4) |
| Child opaque painted pixels at device scale 1 | x=25..28, y=10..13 |
| Hit at the child client center (27,27) | outer |

The same client coordinates were measured at device scales 1, 1.25, 1.5, 1.75
and 2. Adding `position:relative; top:50px` to the outer moves the child client
rectangle to y=75 and its opaque painted pixels at scale 1 to y=60..63. A fitting
20px outer is a neighboring case: both child origins are y=5, or y=55 with top.
The native guards retain exact pixels, client rectangles and physical hit checks
as separate assertions. They do not change the immutable reference images.

## Chromium source routes

All sources below are from the official **147.0.7727.50** tag.

* [LayoutBox::QuadsInAncestorInternal](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/layout/layout_box.cc):
  lines 1205–1222 create a quad for every actual physical part, offset from the
  first part, and map it through the source layout object.
* [LayoutBox::PhysicalBorderBoxRect](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/layout/layout_box.h):
  lines 275–277 use zero local origin and the stitched size.
* [PaintLayer::UpdateTransform](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/paint/paint_layer.cc):
  lines 293–305 resolve the source layout box's transform.
* [LayoutObject::MapLocalToAncestor / GetTransformFromContainer](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/layout/layout_object.cc):
  lines 3682–3730 and 3787–3806 apply the source transform and container offset.
* [PaintPropertyTreeBuilder](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/paint/paint_property_tree_builder.cc):
  lines 1340–1343 resolve each physical paint fragment's own reference box;
  lines 3651–3652, 3754 and 3773–3777 apply placement before its local transform.
* [ComputeReferenceBox](https://chromium.googlesource.com/chromium/src/+/147.0.7727.50/third_party/blink/renderer/core/layout/transform_utils.cc):
  lines 38–48 distinguish a physical fragment's local rectangle from a source
  layout box's physical border rectangle.

Open UI's source client collector shares the transform/origin arithmetic but
uses the retained first source anchor and stitched reference size. It projects
all real authored continuations in layout order. Physical paint, hit clips and
the physical inverse-box cache retain their existing fragment transforms. Real
source boxes take precedence over removed-ancestor descriptor fallbacks. The
collector runs during dirty geometry rebuilding; unchanged frames reuse caches.

## Saved evidence identity

Evidence directories are under the retained qualification archive
`openui-v02-qualification-d174ea0b`; the files listed below are immutable inputs,
not replacement render references.

| Receipt | SHA-256 |
| --- | --- |
| native-promoted-fragment-chromium-v4408/receipt.json | fec09f98a9a019234a40e51b5d2d45fffabb6853c5c402ee24ccddfca1378ce5 |
| native-promoted-origin-review-table-border-review-v4410/source-coordinate-client-quads-proposal-v1.json | 50e5f1b86d46adec299700c9e3107d0a50a9502c24621e215fd6bf21fb1d37d1 |
| native-promoted-origin-review-table-border-review-v4410/primary/receipt-transform-utils-v1.json | d22513eb3b34473653d84e5c78f9d8daac1395b05a180b20f097e1bc5b2ffe85 |
| native-promoted-origin-review-table-border-review-v4410/primary/receipt-layout-mapping-v1.json | 0225db34cfca031ebb8342400343097e0c54b6757e373920ababa069503af213 |

The Chromium receipt binds 25 successful captures, exact fixture/PNG hashes,
Chrome binary SHA-256
`0ba87684df0afc11faaea40ac57bd2f1d68b67191443b38215a36b90e6f08848`
and capture harness SHA-256
`97444384d43fd032a13f71c2b5c8330b6bc7533157f5af17ffc93488a341e321`.
The proposal verifies all 25 retained PNG hashes and records separate image
regions, DOM rectangles, hits and source identities. It did not run a native
pixel comparison.

## Required remaining checks

This source repair is uncompiled and unqualified until the integrated native
and Chromium gates run. The nested reduced input retains a separate layout
failure: Chromium's first in-flow inner part is 38px; the preceding native
principal continuation is 27px. Full unsliced source-border geometry does not
authorize borrowing a positioned child's 38px source window. That measured
client assertion remains a required test, with no ignored case or tolerance.

Nested fragmentation, writing modes, variable fragmentainers, spanners,
repeated table fragments, transformed scroll ancestors and missing retained
source ancestors need their own measured qualification. Reveal and accessibility
consume the public bounds cache and require operation-specific review; the
physical inverse-box cache is unchanged. Exact paint/hit results for the nested
case remain required. No full renderer or release qualification follows from
this geometry source checkpoint.
