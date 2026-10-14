# Native layered paint diagnostics

This is an opt-in static diagnostic source capture. It is not qualified for
release, independently composited surfaces, or presentation-owned animations.
Open UI executes no JavaScript. Chromium remains the rendering and behavior
target; canonical Open UI pixels are a neighboring guard, not the oracle.

## Native operations

`Engine::set_layered_paint_capture_enabled(true)` requests capture on the next
visual update. `SceneSnapshot::layered_paint_capture()` returns an owned immutable
capture. `SoftwareCompositor::render_layered_diagnostic()` explicitly replays it
through the existing CPU raster policy. Ordinary rendering remains canonical.
Changing the flag invalidates paint only. Assigning the same flag and requesting
an unchanged scene preserve the existing zero-work branch and capture identity.

The paint program is partitioned into intrinsic owned draw sources, keyed by the
same nonreused Engine scene role IDs and live document/index/generation identities
as retained metadata. A source can contain multiple runs separated by other
sources. The global sequence retains every original save, restore, transform,
clip, layer, opacity barrier and draw position. Replay redirects only draw
lookups. It does not apply copied style effects a second time or reconstruct
paint ancestry from DOM parents. The same state interpreter serves unpartitioned
and partitioned diagnostic replay, including caller matrix/clip/save preservation.

Actual fragment and contents scopes retain layout provenance. Table helpers,
text runs and anonymous fieldset content inherit their enclosing authored source;
they do not acquire duplicate element effects. Fieldset legend/content structural
clip and scroll membership remain represented by their actual state operations
and the separate retained scene roles. Repeated continuations keep one lifetime
identity while retaining separate source runs and paint scopes.

Each source owns its draw resources and a font-cache lease. Published captures
retain no mutable DOM, Engine reference or callback. A saved scene may be replayed
after Engine destruction and on another thread. The diagnostic frame cache binds
both scene generation and immutable capture identity, separately from the
canonical cache.

## Open gates

`UnqualifiedStaticReplay` is unconditional. Complete operation capture is
separate from layer coverage and admission. Nested Picture operands remain
opaque; image operands are owned but not classified as ordinary resource images
versus rasterized subtrees. Canvas queries remain fixed captured decisions.
The original viewport scrolling-content source remains owned and explicitly
reported. These operands are not admitted as independently movable layers.
Experimental intrinsic pictures retain their unqualified cull-bound status and
are never substituted into replay. Damage remains conservatively full viewport.

The new source guards are uncompiled and unexecuted at this checkpoint. Required
next gates include same-source canonical/typed/layered CPU comparisons, complete
pinned Chromium exact matrices and preservation of historical archive bytes for
integrity. Historical Open UI screenshots are not a correctness oracle for new
renders. Recursive offscreen capture,
proven independent source/cull bounds and transparent layer composition, tight
effect damage, presentation-owned tracks/worker lifetimes and the blocked-UI
animation/performance/hardware gates remain required work. No independent layer
or animation qualification follows from source IDs, tags, or passing static
comparisons. App-facing policy conveniences can be added after the shared source
and replay route is verified; current native consumers can use the public Engine.

## Nested viewport provenance

When capture is enabled, the viewport scrolling picture also retains a typed program observed on its actual recorder canvas. `viewport_scrolling_content()` exposes that child and `recording_programs()` enumerates the recording hierarchy. Engine binds the child to the existing retained viewport recording identity. Authored element IDs are shared consistently across recording streams; borrowed fieldset and table scopes keep their actual roles.

The parent still replays its original opaque picture, with the same clear, tiling, clip, scroll and effect order. Child metadata is never spliced into that parent. The hierarchy retains owned font resources and reports queries, images and opaque picture dependencies conservatively. It does not qualify independent transparent surfaces or dynamic tracks. The additive viewport source role requires downstream exhaustive Rust matches to handle it.

## Shader dependencies

Coverage retains three distinct raster dependency counts: explicit Image/ImageNine
operands, Paint shaders that Skia identifies as backed by one Image, and all
other shader graphs. A rectangle with an image shader retains its raster resource
through the owned Paint. Other shaders, including gradients or composite graphs,
remain conservative dependencies; a non-image result does not prove the absence
of image children. These counts do not admit independent replay or dynamic tracks.

The native resource guard requires the actual explicit PNG Image path at 1x and
the actual PNG-owner image shader path at fractional scales. SVG keeps its exact
teal pixels and recorded owner provenance. Raw SVG helper calls remain an incomplete
source dependency; the opaque parent Picture preserves their canonical pixels.
Owner/chunk presence does not establish complete, independent SVG operand capture.
Actual SVG facade capture or fail-closed source extraction remains required for
transparent source replay. The owned canonical and shader/raster operands must
survive Engine destruction and replay from another thread. All new shader
classification guards are source-only until Root runs the current-source gates.

PaintLayerCoverage has two additive Rust fields and PaintShaderDependency is an
additive typed inspection API. Downstream struct literals must include these
fields; no C layouts or exported ABI symbols change.


## Transparent effect-source diagnostics (source candidate)

Actual painter decisions now emit immutable transform, overflow-clip, clip-path,
filter, mask and opacity projections. A projection uses the real retained source
generation and layout role. A removed transform ancestor with no retained layer
ID remains explicitly unbound; it does not borrow its child's identity. A cell
background clip can retain the cell owner while the active draw scope belongs to
its table. Fieldset content clips retain the wrapper source and their separate
content role, with no additional authored opacity. These annotations do not
change the canonical canvas calls.

`LayeredPaintProgram::transparent_surface_program` owns the actual opacity
intervals, original draw operands, independent world-baked source programs and
pictures, font bytes and cache lease. The selected diagnostic reason is the
actual painter opacity interval with a will-change-transform decision. It is
not a Chromium layer-admission assertion. Ancestor matrices and save depths
remain in each source. Ancestor clips/effects stay on the original parent spine
and apply once. Ordinary intervals replay unchanged; selected intervals use
explicit transparent N32 targets and compose the owning alpha once, including
nested selected intervals. Allocation happens before output replay. Full caller
bounds are the only reported damage; tighter independent culling is unqualified.

Incomplete raw captures, hidden opaque Pictures, image/mask filters and unknown
shader graphs fail closed for extraction. Original SVG/Picture pixels remain
available through canonical replay; SVG owner presence is not complete operand
capture. Fixed canvas queries remain visible dependencies. The source pictures'
cull rectangles, arbitrary host movement, text/LCD policy, tile phase and color
space remain unqualified. Typed source replay retains original operands and
does not substitute the experimental source Picture.

The first API operates on the actual standalone recording program, including
the separately retained viewport child program. It does not yet replace that
child's opaque Picture in its parent, compose a complete tiled document, select
a release renderer path, or publish presentation tracks/worker clocks. Those
remain required follow-up work. Static native guards and exact current Chromium
RGBA gates must pass before any selection is admitted. Historical archives stay
intact for history; their known-wrong bytes are not a correctness requirement.
