# Native column paint and input ordering

Open UI constructs and updates these documents through public native Rust
methods and Rust callbacks. Open UI executes no JavaScript. The separate
pinned Chromium process supplies geometry, hit targets, and reference pixels.

## Shared behavior

A following ordinary block's background must remain behind overflowing atomic
flex content. The shared column decoration pass now includes that background.
Positioned elements and groups with opacity, transforms, clipping, masks, or
paint containment retain their own paint phase.

The engine orders input entries by the corresponding child paint phases.
It collects layout rectangles in their original order, then orders only the
input entries. This preserves owned geometry snapshots while making the
element under the pointer agree with the content painted above it.

The [public Rust application](../../bindings/rust/openui/examples/native_column_paint_phases.rs)
constructs the document with typed styles, queries its owned rectangles,
activates the overflowing child, and observes one Rust click callback. It
changes the following block between static and relative positioning and
between opaque and half opacity, querying the pointer target after each
mutation. The earlier owned rectangle snapshot stays unchanged.

The [C geometry consumer](../../examples/c_v02/geometry.c) exercises the same
mutations and shared engine through the existing ABI. These corrections add
no C exports and change no C struct layouts. The old combined library fails
the new overlap assertion with `SIGABRT`; the rebuilt candidate passes it.
The earlier Rust implementation also fails the public overlap assertion.

## Measured native states

The preserved static inputs contain eleven states at five device scales:
1, 1.25, 1.5, 2, and 3. Of the 55 comparisons:

- 35 match Chromium in complete geometry;
- 38 have exact pixels;
- 45 return the same pointer targets;
- 35 match in geometry, pixels, and pointer targets together.

The following ordinary block and four positioned/effect neighbors match all
three measurements at every scale. The baseline and next-item states also
match all three. Fresh Chromium captures agree with the earlier captures;
the input-order correction changes neither native geometry nor native pixels
from the paint-only prototype.

The existing constrained-box sweep keeps all 65 native images and all owned
geometry unchanged: 60 remain exact in both geometry and pixels. All 144
atomic-child neighbors remain exact and unchanged. The existing 55-state
column-flex application retains all geometry and gains five exact images,
one following-block state at each scale. Its other fifty images remain
unchanged, and no previously exact image regresses.

## Remaining work and qualification

Row flex and grid continuation geometry and pointer targets remain owned by
`openui-layout` and the engine's fragment input traversal. Reversed flex
continuations and the following inline-block state still have geometry or
pixel differences owned by `openui-layout` and `openui-paint`.

This correction does not establish complete CSS stacking, preserve-3d, or
all native element API behavior. Clean complete original and expanded
Chromium matrices, reviewed residual ownership, and the remaining native API
and release gates are still required.

The [candidate evidence index](../v02/generated/native-column-paint-phases-v1.json)
records completed focused and primitive matrices, exact at all 40 profiles.
The wider 1,920-case column/flex selection is 7,034/7,680 exact, with 646
differences and zero errors. All 9,280 candidate images and Chromium oracle
identities match the preceding checkpoint, with no formerly exact regression.
Candidate measurements from the development checkout are diagnostic evidence;
they are not clean-source release qualification.

The accepted clean `056421db` checkpoint now passes
[640/640 focused and 960/960 primitive comparisons](generated/focused-primitive-raster-v50.json)
at all 40 profiles. All 1,600 Open UI RGBA images, Chromium RGBA images and
oracle identities remain unchanged from `497e322d`. The clean original and
expanded matrices at this source have also finished:
[21,291/22,924 original comparisons](generated/four-profile-census-v48.json) and
[22,094/23,728 expanded comparisons](generated/expanded-requalification-v31.json)
are exact, with zero errors. The
[complete delta](generated/native-column-paint-phases-full-delta-v1.json)
verifies every Open UI image, Chromium image and oracle identity unchanged
from `497e322d`; all original rows agree between the full runs. Both commands
returned exit 1 for their remaining pixel differences. This evidence does not
close the wider renderer, native interaction or release gates.
