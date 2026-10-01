# Native vertical maximum-size overflow

Open UI builds and mutates these documents through native Rust operations.
Applications use public Rust element methods and callbacks; Open UI never
executes JavaScript. Chromium 147.0.7727.50 is the separate geometry and pixel
reference. Earlier inputs, captures and results remain preserved.

## Reviewed cause and correction

The multicolumn overflow checks read physical `max-height` when deciding
whether an element's visible children contribute to balancing. In vertical
writing, the block axis uses physical width, so a `max-width` constraint was
missed. Repeated measurement also omitted maximum-size overflow from its
decision. Together, those checks discarded the child's remaining flow and
balanced 160 pixels instead of the full 206-pixel bordered child.

The shared block-size helper now resolves the maximum size in the parent's
writing axes. Initial measurement, repeated measurement and continuation
distribution use that value. The containing element keeps its own 160-pixel
box while its visible child retains all 206 pixels of source flow. The owner
is `openui-layout` multicolumn fragmentation. No fixture selection, raster
correction or reference rewrite is involved.

## Chromium measurements

The [diagnostic index](../v02/generated/native-vertical-max-block-v1.json)
records both `vertical-lr` and `vertical-rl` at scales 1, 1.25, 1.5, 2 and 3.
All **10/10 comparisons** now match every queried node's geometry and have
zero different pixels. The balanced width is 68.671875 pixels. The containing
element's final fragment is 22.65625 pixels wide; its child keeps a 68.65625-pixel
final fragment. The latter can receive pointer input outside the containing
element's own border box.

The complete reduced sweep has **53/65 pixel comparisons exact** and
**45/65 complete geometry comparisons exact**, up from 43/65 and 35/65.
All 55 horizontal images and complete geometry records are unchanged.
The preceding 144 atomic-child states are also byte-identical and retain
exact Chromium geometry and pixels. Cloned decoration, constrained flex,
flow-root child geometry and entirely empty bounding rectangles remain
explicitly different; no remaining case is counted as passing.

## Public native consumers

The Rust application creates both vertical documents through public
`Document` and `Element` methods. It queries owned client rectangles, checks
the balanced width and hits the overflowing child in the last column.
Its new guard fails against the preceding source and passes with this fix.
All 55 application scenarios pass with Linux enabled.

The native C consumer builds the same two documents, reads their owned
fragment lists and verifies the borrowed hit-test handle identifies the
child. Its new guard rejects the preceding library. All 109 exports and
prior struct layouts remain unchanged; six C consumers and the C++ header
consumer pass. The locked workspace passes 8,491 tests, with 13 ignored
across 147 suites. Formatting and read-only generated contracts pass.

These reduced measurements use development source and have
`release_qualification: false`. Complete clean original, expanded, focused
and primitive matrices, reviewed residual ownership, remaining native API
coverage and release-lab gates still need to pass. The completed clean
`547c7081` renderer totals describe the earlier checkpoint.

## Reproduce

Build the native probe with the pinned configuration:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml build --locked \
  -p openui-engine --example constrained_box_geometry
target/debug/examples/constrained_box_geometry \
  ../../out/native-vertical-new/lr vertical-lr 1.25
target/debug/examples/constrained_box_geometry \
  ../../out/native-vertical-new/rl vertical-rl 1.25
```

Use new output directories. The preserved static inputs in
`docs/v02/evidence/native-constrained-box-v1` belong to the separate Chromium
oracle. Native applications perform every interaction through the framework.
