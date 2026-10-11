# Native table geometry and border findings

## Current ownership and opacity checkpoint

The new [ownership and opacity measurement](native-table-ownership-opacity.md)
records **25/25 exact pictures**, **275/275 geometry records** and **250/250 hit
checks** at clean `c4e8a809`. A separate 720-case repeated CPU diagnostic explains
the measured N32/F16 opacity distinction. Layout and native conformance pass;
all 170 paint tests pass after correcting authored-box test setup at `0d178e2c`.
The last complete original census remains 21,329/22,924 exact; new focused,
primitive and complete runs are still required.

## Historical table-origin diagnostic at `c6a0b92e`


This is a reduced diagnostic, measured from clean `c6a0b92e`. It does not
qualify the renderer or complete the native geometry APIs. Pinned Chromium
147.0.7727.50 defines expected pixels and geometry. Open UI executes no
JavaScript; the consumer calls public Rust methods and a Rust callback.

## Reproducer and measured results

The [public Rust consumer](../../bindings/rust/openui/examples/native_table_origin.rs)
constructs a floated collapsed table, a split caption, one cell and an ordinary
text block at the caption's measured horizontal origin. The compiled consumer
is preserved unchanged as that example. Its dimensions come from public
`Bitmap::width()` and `height()` methods; the first failed build that tried
private fields is retained in the evidence.

CLI: `OUTPUT WIDTH HEIGHT SCALE auto|fitted no|yes [TABLE_WIDTH]`.
The optional final argument sets an authored table width for later experiments;
the completed run used auto width. `no|yes` selects empty baseline markers.

The clean external consumer uses the same retained Engine as native apps.
Forty independent native processes exercise two explicit raster policies,
two marker modes and two repeats at five scales in a 400×240 logical viewport.
Ten independent Chromium processes provide matching captures and geometry.
Sixteen fresh compiler records and 29 compiled paths are verified. The new
independent audit checks all 20 comparisons and 80 original PNG files.

| Scale | Automatic mismatched pixels | Fitted mismatched pixels |
|---|---:|---:|
| 1 | 0 | 0 |
| 1.25 | 562 | 482 |
| 1.5 | 432 | 432 |
| 2 | 0 | 0 |
| 3 | 432 | 432 |

Marker and unmarked images have identical pixels in both native policies and
Chromium, and every common named geometry record is unchanged. Each native
process repeats unchanged frames, dispatches a count-only Rust click callback,
checks that the callback preserves pixels, and verifies weak teardown.
**8/20 images** and **75/170 complete geometry records** are exact. Geometry
records include bounds and ordered client rectangles, so passing pictures
alone does not establish native API correctness.

The reduced sample does **not** reproduce the four original table regressions.
Their immutable fixtures and results remain required guards. Neither raster
policy is promoted by this measurement.

## Confirmed native geometry gaps

With fitted metrics, the table's bounding rectangle agrees with Chromium:
`(x=286, y=0, width=114, height=34)`. Native `client_rects()` returns three
rectangles: the outer table and two copies of the inner 18-pixel grid extent.
Chromium returns one outer rectangle.

Source tracing explains the duplicates. Layout gives the authored table
wrapper, synthetic grid-decoration box and anonymous row the same table node
identity. The Engine collector accepts every nontext `Box` with that identity.
Its traversal also reapplies the node's authored transform, clip path, hit
eligibility and child scroll translation to those helpers. Those repeated
effects are established by source inspection; transformed and scrolled
consumer comparisons are still required.
Row-group collection can also reuse a group node for anonymous row/cell
content while constructing an authored outer group box. Those helpers need
the same ownership classification and row-group guards.

The collapsed cell has a separate used-box discrepancy. Native reports
`(286, 16, 114, 18)` while Chromium reports `(286.5, 16.5, 113, 17)`. The native
query exposes paint extent. Layout already supports a separate
`principal_box_rect`; its cell value must come from resolved winning collapsed
edges. Halving every authored border width would not handle conflicts or spans.

The automatic policy also retains the earlier fractional intrinsic-width gap:
its table width is `114.015625` and x is `285.984375`, versus Chromium's `114`
and `286`. The explicit fitted policy restores these metrics. Its remaining
paint and used-box differences still fail qualification.

## Collapsed-border paint hypothesis

Exact 147.0.7727.50 `table_painters.cc`, `box_border_painter.cc` and the relevant
geometry source are preserved with URL and byte receipts. All three also match
the local 147.0.7727.24 source bytes; the local patch version was not assumed to
be the oracle version.

Chromium resolves grid edges and their joints, constructs each completed edge
rectangle, snaps it and paints it independently. Equal perpendicular winners
can both cover their joint. Native collapsed segments retain border boxes and
struts and pass through the generic border painter; a uniform solid box can
reach one centered stroke.

Existing immutable corner pixels support this difference. One half-covered
blue draw over white leaves a channel near 127; two independent half-covered
draws leave a channel near 63. Both values occur in the original comparison.
This arithmetic and the pinned source support a joint-overlap hypothesis.
They do not replace a reduced capture and actual edge-operation trace.
Coverage packing and channel rounding remain open.

## Required shared repair and guards

Layout must distinguish authored element boxes from formatting and decorative
helpers while retaining their style source. Geometry, input and paint must use
that ownership consistently for authored effects and scrolling. Helper offsets,
structural clips and descendant traversal remain part of the scene. Real
inline and multicol continuations remain distinct rectangles, including empty
ones; deduplicating by node, equal rectangle or containment would be wrong.

Collapsed cells need their used authored box separately from paint extent.
Collapsed painting needs resolved grid-edge winners, widths, styles, colors and
joint endpoints. A shared grid edge is painted once; equal perpendicular joint
winners retain their independent overlapping draws. Ordinary boxes and
separated tables retain their appropriate contour behavior.

Verify transformed, scrolled and clipped table consumers, real inline/multicol
continuations, row-group helpers, detached queries, empty bounds, singular
transforms and input eligibility. Then sweep the original table/caption phases and run the required
focused, primitive and complete Chromium matrices. No fixture, font, color,
scale or output-pixel exception is a repair.

[Immutable source, captures, geometry, compiler records, failed build and audits](generated/native-table-origin-v1.json).
The default raster policy and prior references are unchanged. The latest full
original census remains **21,329/22,924 exact**; all remaining rendering and
needed native API work stays open.
