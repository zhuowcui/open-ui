# Native image fallback and resource clearing

Open UI executes no JavaScript. Applications supply image resources and handle
interaction through public Rust methods and Rust callbacks. Chromium is the
sole pixel target, with zero tolerance.

## Shared correction

The earlier inline image candidate `c92e2d08` is rejected because it sends an
image with retained fallback children through atomic replaced layout. One
expanded case loses exactness at all four profiles. The
[terminal review](generated/native-review-v3.json) preserves those failures.

Fresh private `7d09f7a1` keeps an image's fallback children in normal inline
flow when there is no installed replaced resource. An installed resource still
uses atomic inline layout, including authored and intrinsic dimensions. Empty
image hosts and the canvas, SVG, audio and video defaults retain their existing
classification. This is shared layout behavior; no fixture or reference bytes
are changed.

The same candidate adds `Element::clear_image_resource` over the shared Engine
and append-only C export `oui_element_clear_image`. Clearing retains styles,
children and the document's registered resource bytes. Repeated clearing does
not invalidate an already clear document. Applications can install and clear
image content from native event callbacks. All 113 existing C exports and all
30 struct layouts remain unchanged; the candidate has 114 exports.

The source includes Engine guards and Rust/C/C++ consumers for resource
installation, clearing, retained fallback children, owned geometry, repeated
clearing, detach/reattach, callback lifetime and teardown. These native APIs
remain unapplied and unqualified until their application checks pass.

## Measured Chromium geometry

Two independent Chromium runs agree on all 120 ordered queries: five scales,
three subpixel origins, and before/load/clear/reattach states. At origin
`(20, 20)`, the fallback host is `100 × 100`; loaded image content is `300 × 200`;
clearing and reattaching restore `100 × 100`. The source image is the unchanged
200 × 200 resource used by the earlier native inline consumer.

Thirteen read-only checks pass, including generators, accountability, Rust
formatting and C/C++ syntax. These checks and Chromium queries generate no
native pixels and do not prove native geometry or pixel equality.

## Pending execution

The [preserved preparation](generated/native-inline-fallback-v1.json) includes
source patches, consumers, immutable queries, read-only checks and verification
probes. The first prepared queue was never launched: its copied metadata
incorrectly described the additive C ABI as unchanged. A fresh source root and
new probes preserve that preparation and record the append-only ABI accurately.

Exclusive whole owner `1576` waits for all 29 prior pipelines, including every
remaining stage of raster owner `1560`. It then requires the named baseline
assertion failure, fixed fallback guard and resource/ordinary-inline neighbors,
clean locked workspace, native consumers, 120 exact application images, and all
four complete renderer matrices. Two native runs, 240 independent Chromium
processes and 480 consecutive Chromium captures must agree. Native execution
and all pixel gates remain pending. Accepted renderer totals and release
admission are unchanged.
