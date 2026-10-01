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
states. A separate N32 opacity prototype improves that suite to 124 exact
states; it is still under investigation and is not part of this correction.

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
