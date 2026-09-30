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
every rectangle is empty, it returns the first. If there is no layout box,
it returns `None`. `width()` and `height()` read these combined bounds.
These are owned native snapshots; later mutation does not change an earlier
returned rectangle or list.

The engine records geometry independently of the pointer hit-test list during
the same retained fragment traversal. Accessibility bounds and view-timeline
geometry read the same combined bounds. Static painting is unchanged.
The rectangle semantics follow the
[CSSOM View geometry operations](https://drafts.csswg.org/cssom-view/#dom-element-getboundingclientrect)
for the measured cases; this does not claim the entire Web API surface or
complete 3D qualification.

## C consumer

The append-only `oui_element_get_client_rects_v1` export copies the same
rectangles into caller-owned storage. A null destination and zero capacity
query the required count. If capacity is too small, the count is updated and
the destination is untouched. Layout can change between count and copy calls.
Existing C struct layouts and the 84 frozen exports remain intact; there are
107 current exports. The existing `oui_element_get_bounds` now returns the
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
They belong to the complete 54-scenario application conformance suite.

Reproduce the Chromium measurement into a new directory:

```sh
python3 tools/qualification/probe_native_geometry_oracle.py \
  --results-dir out/renderer-evidence/native-geometry-new-capture
```

The tool executes queries only in the separate Chromium oracle and preserves
each profile and result. Open UI applications and tests perform their element
operations through native Rust or C. These measurements do not qualify the
complete pixel matrix, all remaining native API behavior, or the release lab.
