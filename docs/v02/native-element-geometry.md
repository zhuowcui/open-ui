# Native element geometry

Applications query geometry directly through public Rust methods. Open UI
executes no JavaScript. Chromium remains the separate reference used to check
the results. The [diagnostic index](generated/native-geometry-diagnostic-v1.json)
records source and report hashes, baseline failures, final workspace results,
the C consumer gate, and the fresh Chromium recheck.

## Public operations

`Element::client_rects()` flushes pending layout and returns an owned
`Vec<Rect>` for the element's border-box fragments in layout order.
Coordinates are logical viewport pixels after ancestor scrolling and
transforms. Clipping, visibility, and pointer eligibility do not erase a
layout box. Empty boxes and collapsed transforms retain their coordinates.
Detached elements and `display: none` elements return an empty list.
Destroyed handles return an error.

`Element::bounding_rect()` combines the nonempty fragment rectangles. If
every rectangle is empty, it returns the final rectangle, matching Chromium's
ordered union. If there is no layout box,
it returns `None`. `width()` and `height()` read these combined bounds.
These are owned native snapshots; later mutation does not change an earlier
returned rectangle or list.

The engine records geometry independently of the pointer hit-test list during
the same retained fragment traversal. Accessibility bounds and view-timeline
geometry read the same combined bounds. Geometry storage is separate from
paint clipping and decoration ownership.
The returned geometry is checked directly against pinned Chromium,
including its ordered rectangle union for entirely empty lists. This does
not claim the entire Web API surface or complete 3D qualification.

## C consumer

The append-only `oui_element_get_client_rects_v1` export copies the same
rectangles into caller-owned storage. A null destination and zero capacity
query the required count. If capacity is too small, the count is updated and
the destination is untouched. Layout can change between count and copy calls.
Existing C struct layouts and the 84 frozen exports remain intact; there are
109 current exports. The existing `oui_element_get_bounds` now returns the
combined bounds too.

The [C geometry consumer](../../examples/c_v02/geometry.c) creates a real native
three-column document through public C operations. It checks every column's
coordinates, combined bounds, undersized-buffer handling, hidden and
pointer-ineligible boxes, detachment, reattachment, stale handles, and owned
copies. Its assertions remain enabled in Release SDK builds.
This consumer also exposed missing native C value conversion for column count
and visibility. Both now use the shared native style path. Column count keeps
its existing integer wire tag: zero represents `auto`, positive values specify
the count, and negative values are rejected. C counts must fit `int32_t`.
This parses a typed property literal; it adds no script execution.

## Measured reference

The [preserved Chromium geometry](evidence/native-geometry-v1/chromium-geometry.json)
uses the pinned 147.0.7727.50 binary at a 320×240 logical viewport and scale 1.
Its five static HTML inputs contain no script. A separate fresh recheck made
two agreeing geometry queries for each input and reproduced all five results.
The three-column case gives rectangles at x=0, 108, and 216, each 56 pixels
wide, with heights 68.671875, 68.671875, and 68.65625. Combined bounds are
272×68.671875 at (0,0). The public Rust and C consumers assert these values.

Three public Rust regressions failed before the repair: the column query
returned only x=216, pointer-ineligible elements returned no box, and an empty
box returned no rectangle. Four native geometry scenarios now cover these
cases plus scroll, transform, ownership, accessibility, and handle lifetime.
They belong to the complete 55-scenario application conformance suite.

Reproduce the Chromium measurement into a new directory:

```sh
python3 tools/qualification/probe_native_geometry_oracle.py \
  --results-dir out/renderer-evidence/native-geometry-new-capture
```

The tool executes queries only in the separate Chromium oracle and preserves
each profile and result. Open UI applications and tests perform their element
operations through native Rust or C. These measurements do not qualify the
complete pixel matrix, all remaining native API behavior, or the release lab.

## Constrained boxes and visible child overflow

A box with a fixed or maximum height can have a taller child that continues
through later columns. The public geometry query previously reported that
child flow as the containing box's own size. Layout now carries a separate
local border-box rectangle for that continuation. Geometry, pointer input,
accessibility bounds, and view timelines read the same owned box; children
keep their independently positioned overflow fragments. Paint clipping and
decoration budgets remain separate layout data.

For a 160-pixel box containing a 206-pixel bordered child, Chromium gives the
box's last column a height of 22.65625 pixels and the child's last column a
height of 68.65625. The public Rust and C consumers now assert both values.
They also shorten the box to 120 pixels and check its zero-height final
continuation, 192-pixel combined width, unchanged child flow, and owned
rectangle copies. The Rust consumer checks hit testing outside the box and
on the visible child, updated accessibility bounds, zero-height boxes, and
minimum height taking precedence over maximum height. A clipping-descendant
guard also exposed missing child continuations: a normal block with its own
used size was dropped after its atomic child ended. Layout now retains and
slices that independently owned block space. Synthetic input clips use the
continuation extent that carries child flow; authored clips use the own box. The Rust and C checks
both fail against the preceding implementation and pass with the repair.

The [constrained-box diagnostic](generated/native-constrained-box-geometry-v1.json)
records 13 reduced static inputs at scales 1, 1.25, 1.5, 2, and 3. The box's
client rectangles match Chromium in 45/65 comparisons; all three queried
nodes' complete geometry matches in 35/65. Pixels match in 43/65. All 13
scale-1 native images are unchanged by this geometry repair. These development
measurements do not qualify the full renderer or all native APIs.
An additional clipping-descendant reproducer now matches all three nodes'
geometry and retains native pointer interaction in later columns. Its pixel
difference decreases from 16,074 to 11,358 and remains open. The broader
raster diagnostic predates this child-retention correction; fresh complete
matrices are still required for the final source.

The remaining cases are explicit:

- Cloned borders and padding produce the wrong continuation sizes and count.
- A constrained flex column does not fragment its visible child flow.
- Vertical maximum-size overflow is omitted from column balancing.
- A flow-root wrapper retains whole child boxes behind clips; its child
  geometry differs and fractional border pixels differ at scales 1.25 and 1.5.
- The additional clipping-descendant case still differs in painting.
- For three entirely empty continuations, the published native bounds
  operation returns the first rectangle; the measured Chromium operation
  returns the last. The native contract remains explicit. Its client
  rectangles and pixels agree at all five scales.

The complete locked workspace passes 8,491 tests, with 13 ignored; the Linux
engine/framework/C/platform suite passes 193 tests, with eight ignored. The
C ABI consumer gate preserves all 109 exports and passes six C consumers plus
the C++ header check. The 244 Python checks pass. Complete original and
expanded renderer rechecks for this source remain required.

Build and run the native reduced state:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml build --locked \
  -p openui-engine --example constrained_box_geometry
target/debug/examples/constrained_box_geometry \
  ../../out/native-constrained-new/max160 max160 1.25
```

Measure the preserved static input in the separate Chromium oracle:

```sh
python3 tools/qualification/probe_native_geometry_oracle.py \
  --suite constrained --scale 1.25 \
  --results-dir out/native-constrained-new/chromium-1.25
```

Both tools preserve earlier results by requiring a new output directory.
The Chromium tool can take an explicit pinned binary through `--chrome`.
Open UI constructs and changes every state with native Rust operations.

The later [atomic descendant candidate](../renderer/native-monolithic-column-balancing.md)
repairs the reduced clipping-descendant pixels and its unconstrained column
balancing. Its 40 native states are exact at five scales; three larger child
cases in an additional constrained-column sweep remain different. Wider
renderer qualification for the candidate is still required.

The newer [atomic-child deferral candidate](../renderer/native-atomic-column-deferral.md)
matches 144 reduced native states against Chromium, including constrained
columns, neighboring heights and nested padding. Public Rust and C consumers
perform those mutations and query the resulting owned rectangles. The prior
65-case constrained sweep remains byte-identical and retains its existing
failures. Complete renderer and native API qualification remain open.

The later [vertical maximum-size correction](../renderer/native-vertical-max-block.md)
uses the physical width bound for the vertical block axis and preserves
visible child overflow through balancing and continuation distribution.
Both vertical directions match Chromium in complete geometry and pixels at
all five scales. The reduced sweep now has 53/65 exact pixels and 45/65
exact complete geometry; all 55 horizontal images and geometry records and
the 144 atomic-child states stay unchanged. Public Rust and C consumers query
those fragments and hit the overflowing child in the last column. Their
guards reject the preceding implementation. Complete clean renderer and
native API qualification remain open.

The later [flow-root child-slice correction](../renderer/native-flow-root-leaf-slices.md)
materializes ordinary child continuations inside independent block formatting
contexts. All five flow-root states now match Chromium in complete geometry
and pixels, including the fractional border phases. The reduced sweep has
55/65 exact pixels and 50/65 exact geometry; all 60 other states are unchanged.
A consuming Rust application uses only public framework methods, exports
exact geometry and pixels at all five scales, hits the last-column child and
runs a Rust click callback once. The C consumer queries the same fragments.
Those guards reject the preceding implementations. Wider renderer and native
API qualification remain required.

The later [entirely empty bounds correction](../renderer/native-empty-fragment-bounds.md)
uses Chromium's ordered rectangle union instead of preserving the former
first-fragment fallback. Rust and C consumers now assert the final empty
column's position, and the Rust application verifies matching accessibility
bounds and owned snapshots. All five zero-height reduced cases match
Chromium in complete geometry; every one of the 65 PNGs stays unchanged.

The combined [column-flex and empty-bounds checkpoint](../renderer/native-column-flex-overflow.md#combined-native-checkpoint)
now matches the original reduced sweep in complete geometry and pixels for
60/65 comparisons. Its public native Rust application matches both for eight
case variants at all five scales. All 144 atomic guards remain exact; row/
reversed flex, following-block paint order and cloned decoration remain open.
