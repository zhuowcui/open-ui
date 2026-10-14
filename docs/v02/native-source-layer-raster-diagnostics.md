# Native source-layer raster diagnostic

This optional CPU renderer is an **unqualified static diagnostic**. Enable the
existing layered capture, obtain an immutable scene, and explicitly call
`SceneSnapshot::with_source_layer_diagnostic()` followed by
`SoftwareCompositor::render_source_layer_diagnostic()`. The ordinary renderer
continues to select its original canonical recording.

The painter records actual background, foreground, outline and replaced-item
boundaries separately from element and layout-helper ownership. The plan keeps
the original ordered operations and owned draw operands, plus generation-bound
containing-block and scroll-owner identities. Transform, clip, effect and scroll
projections refer to those exact operations and their parent intervals. Native
rounded paint offsets and residuals are copied; fixed position alone never
discards a residual or establishes the final Chromium material.

Candidates require an actual view containing block, the existing internal
transform hint, or paint isolation. This does not add a public authored
`will-change` property. Borrowed fieldset/table/column/text helpers keep their
real role and structural state without acquiring another authored effect.
Siblings retain operator order and overlap barriers. Only contiguous projections
of the same source and compatible entry state can merge; multiple items and
transform upcasts revoke a single-record solid annotation.

A balanced owner translation around exactly one eligible background `DrawRect`
can remain a direct composition property. Its original rectangle, color and
property identity are retained separately from the translation matrix; the
renderer never draws a transformed bounding rectangle. The drawing item must
still contain exactly one record operation. Internal scale, rotation, skew,
clips, effects, queries, mixed transforms and additional draws keep the existing
picture or unavailable route. A retained direct property prevents merging across
that boundary, while the complete original operation spine remains available.

`SolidColor` requires a single real background record operation with a supported
covering rectangle and simple paint flags. Rounded records never become solid.
`OpaquePicture` requires actual opaque coverage of the entire source bounds and
an opaque background hint. A background hint alone is insufficient.
`TransparentPicture` retains transparent clear and original draw AA/alpha.
All three strategies remain unqualified.

Each admitted local recording removes only its paint-coordinate placement and
rebases save-depth commands. Ancestor clips and effects remain on the parent
spine; internal clips/transforms/alpha stay in the source, apart from the proven
single-record owner translation described above. Local physical raster
keeps fractional phase and the pinned `CalculateClearForOpaqueRasterRects`
boundary preclear. External sampling texels are allocated separately and are
not exposed as frame ink. Quad AA depends on the actual device transform and
translation; no original background paint or global AA flag is changed.

An explicit reference replaces exactly the parent's original viewport Picture
draw with its complete child recording. Incomplete SVG/raw-canvas recordings
remain original opaque Picture dependencies and are reported by
`unavailable_recordings()`. Unknown pictures, shader graphs, unsupported bounds,
three-dimensional transforms and query-dependent intervals are unavailable for
independent extraction. Retained resources and font leases remain owned after
the Engine is destroyed. No DOM borrow or callback crosses the publication
boundary.

Publication and CPU cache are separate from canonical and exact-spine replay.
Repeated immutable scene/capture identity reuses the completed frame without
new source raster or readback. Damage remains the full viewport. This diagnostic
uses an explicit standalone untiled target; canonical tiling, tighter damage,
worker sampling and presentation-owned animation tracks are separate gates.

Correctness requires the current Chromium 147.0.7727.50 original fixed-edge
witness and all eight neighboring controls at five scales, exact entire RGBA
and geometry, plus the existing ordinary/effect/clip/text/resource gates.
Observed cc owner-layer classes do not prove executed final Viz quad/material
decisions. Compilation, native runtime and those pixel gates are pending for
this checkpoint. Historical archives retain their integrity; known-wrong old
native screenshots are not correctness targets. No canonical renderer, native
API completeness, dynamic-layer or release qualification follows from this
source checkpoint.


## Viewport-fixed overflow ownership

The pinned Chromium view overflow calculator excludes a fixed direct child. Native layout now uses the actual owned positioned containing-block metadata for this distinction, shared with immutable paint context classification. Missing metadata remains unresolved; a fixed child of a transformed containing block contributes through its ordinary ancestor. The viewport's own seed and ordinary/absolute overflow retain their existing rules.

The unchanged tall-sibling control is not a form control. Its Chromium vertical scrollbar paints gray over the right end of the blue edge; the old full-blue-row premise is preserved separately. Fresh current-source metrics also exposed an extra native horizontal scrollbar before this repair. This source checkpoint does not alter pixel expectations, drawing operations, default renderer selection or qualification status. Actual runtime and full Chromium frames remain required gates.
