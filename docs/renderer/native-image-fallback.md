# Native image fallback rasterization

Open UI runs no JavaScript. The consuming native Rust app constructs these
states through public `Document` and `Element` methods. A separate pinned
Chromium 147 process supplies the reference pixels and geometry.

## Shared correction

The renderer now samples the failed-image resource through Chromium's legacy
software image coordinates at every tested device scale. Each analytic
coverage span starts its own inverse-matrix calculation, then advances in
32.32 fixed point. Sampled premultiplied colors retain their original bytes;
the draw applies image coverage and overflow-clip coverage separately using
the packed rounding rules from Chromium's pinned Skia source.

The failed-image shadow frame paints before its icon. Its authored percentage
width resolves against the image host, with the minimum extent required by
its borders and padding. Fallback decoration origins snap in CSS paint space,
while public layout rectangles retain their original fractional coordinates.
An overflowing monolithic child's background paints once with that child's
column replay.

These rules operate on shared paint and resource data. They contain no test
IDs and do not alter captured Chromium images. CPU Skia remains the portable
qualification backend; direct Ganesh raster remains unpromoted.

## Public native measurements

The [image coverage app](../../bindings/rust/openui/examples/native_image_coverage.rs)
uses typed styles, native attributes, public owned bounds and PNG rendering.
Its 24 static states cover two widths, three origins and four backgrounds at
five device scales. All 120 images and owned bounds match Chromium exactly.

An additional 160 states exercise image opacity, parent opacity and transparent
backgrounds through public native setters. The image correction makes 40
states exact, improves 138 comparisons against explicitly rebuilt preceding
libraries, and regresses none. Opacity remains incorrect in the other 120
states. The later [native opacity investigation](native-image-opacity.md)
makes all 160 exact in a shared N32 and layer-origin prototype and verifies
another 160 public Rust states. Additional shadow, outline, transform and
clip cases retain failures, including one worsened comparison. The prototype
remains unapplied pending the shared correction and clean qualification.

The [column image app](../../bindings/rust/openui/examples/native_column_image_fallback.rs)
queries both public bounds and continuation rectangles. Of its 50 states,
41 have exact pixels. Source-less image frames and parent decoration coverage
still differ at fractional scales. Containing column geometry also remains
incorrect: matching a rendered image does not establish correct owned
rectangles or complete native API behavior.

## Verification and remaining work

The candidate passes 640 focused and 960 primitive comparisons at all 40
profiles, preserving every preceding image and Chromium oracle identity.
The final clip-mask candidate completes the wider column/flex selection at
7,039/7,680 exact, with zero errors. Against the preceding checkpoint, eight
comparisons improve and five become exact; none worsen or regress from exact.
Every Chromium image and oracle identity remains unchanged. The
[evidence index](../v02/generated/native-image-fallback-v1.json) preserves the
complete reports, source identities, reviewed isolated causes and rejected
baseline observations. These development measurements
are diagnostic; clean complete original and expanded matrices are required
after the source checkpoint.

The separate complete clean `9b158cda` runs are now indexed in the
[v49 census](generated/four-profile-census-v49.json),
[v32 expanded ledger](generated/expanded-requalification-v32.json), and
[v51 raster index](generated/focused-primitive-raster-v51.json).
The original suite is 21,308/22,924 exact, with 1,616 differences and zero
errors; the expanded suite is 22,111/23,728 exact, with 1,617 differences and
zero errors. The [complete delta](generated/native-image-full-delta-v1.json)
records 30 improved images, 17 newly exact comparisons, no worsened comparison
and no exact regression from `056421db`. All Chromium images and identities
remain fixed. All original rows agree between the suites, and 200 of the 201
additions remain exact at all four profiles. Focused and primitive matrices
remain 640/640 and 960/960 exact with unchanged images. The 892 residual test
IDs still lack reviewed ownership; complete rendering qualification stays open.
Earlier interrupted runs remain preserved as incomplete evidence.

The complete locked workspace suite passes 8,491 tests, with 13 ignored.
Formatting, eight generated-source checks and the accountability audit pass.
The Linux FFI suite passes 26 tests; 239 Python tests and the C/C++ ABI
consumers pass, preserving all 109 current exports and 84 frozen exports.
All 319 earlier constrained, atomic-child, column-flex and paint/input states
retain their native geometry and pixels; the paint/input callbacks and hits
also remain unchanged.
Chromium remains the sole pixel target. The original pixel gate, reviewed
residual ownership, native geometry, opacity, other native operations and
release qualification remain open.
