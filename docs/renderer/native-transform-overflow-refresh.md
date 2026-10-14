# Retained overflow after native transform mutations

The Engine refreshes actual-box transforms and retained scroll areas before
publishing a composite-only scene. Child overflow reaches its real ancestors,
including the viewport. The viewport retains its actual principal layout
rectangle before restoring outer gutters, then combines that own size/padding
contribution with its client rectangle and current propagated descendants.
Restored outside size and previous aggregate content are not replacements for
current scroll content. The additive Rust ViewportScrollport.layout_box_rect
field is initialized by both actual constructors; downstream Rust struct literals
must supply it. C exports and layouts are unchanged.

The same native legal-offset policy clamps shrinking ranges. Changed offsets
queue the existing owning-thread scroll notification and rebuild sticky layout
only when it consumes that scroll offset. Snap targets, button states and
progress timeline snapshots read the updated retained geometry. Sticky paint
movement does not replace its retained normal-flow overflow position.

An auto scrollbar existence change requests layout, because the gutter can
change percentage bases and child sizes. Existing specified-transform containing
block checks request layout for actual fixed/absolute descendant topology
changes. Ordinary two-dimensional transform updates preserve box measurements;
changed projective transforms conservatively use layout. An unchanged scene or
identical setter performs no work.

Pinned Chromium147.0.7727.50 source: LayoutBox::RecalcScrollableOverflowNG;
ScrollableOverflowCalculator::RecalculateScrollableOverflowForFragment;
PaintLayerScrollableArea::UpdateAfterOverflowRecalc. The unchanged ordered
mutation witness was measured at all five scales, twice, in both its original
script input and final template: client200x150, scroll250x150 after scale(4)
about the real100x50 box center. Native scripts are not evaluated.

This source checkpoint has not been compiled or executed. Its ten Engine
guard groups and all existing native, C, pixel and unchanged-frame gates still
require actual Root execution. Broader projective, promoted-fragment and
fragmentation correctness is not inferred from these two-dimensional guards.
No C exports/layouts or immutable Chromium/archive inputs change.
