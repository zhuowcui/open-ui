# Native constrained boxes with atomic descendants

Open UI constructs these states through native Rust Engine operations. It
executes no JavaScript. Chromium 147.0.7727.50 is the separate geometry and
pixel reference. The inputs and captures are preserved; no earlier fixture,
manifest, cached reference or historical screenshot is changed.

## Reviewed causes

A small `overflow: hidden` descendant made paint replay the complete parent
background and border outside its column clip. The parent did not have an
indivisible overflowing box at that boundary. The shared paint query now
checks the descendant's actual source interval before selecting that replay.
Layout also retains the parent's own decoration limit when its child has a
clipping descendant: a child clip does not extend the parent's background.

A larger atomic descendant exposed a separate balancing error. Arithmetic
balancing divided the total flow below its first feasible fragmentation unit.
A constrained ordinary box now propagates that unit, including block-start
decoration and the block-end decoration that finishes its content space.
An oversized atomic child supplies its own minimum rather than turning its
ancestor's overflow offset into more source size. Owned nested multicolumn
contexts keep their own balancing path.

## Measured native states

The [diagnostic index](../v02/generated/native-monolithic-column-balancing-v1.json)
preserves eight static inputs with atomic child heights 0, 20, 40, 68, 69, 90,
200 and 300 pixels. Their constrained ancestor owns 160 pixels and contains a
206-pixel bordered block. At scales 1, 1.25, 1.5, 2 and 3, all **40/40** native
states match Chromium's complete geometry for all three queried nodes and
have **zero different pixels**. The preceding geometry checkpoint matches
none of those pixel comparisons. A paint-only correction matches 15/40;
adding the balancing correction matches all 40.

The five larger child sizes produce balanced column heights 71, 72, 93, 206
and 300 pixels. The public native Rust consumer changes the child height and
checks each resulting owned fragment list. The same scenario checks pointer
interaction and accessibility, then renders the native document and checks
that its small child does not move an ancestor border or background beyond
its column. These guards fail against the preceding implementations.

All 65 images and complete geometry records from the earlier 13-case,
five-scale [constrained-box sweep](../v02/native-element-geometry.md#constrained-boxes-and-visible-child-overflow)
stay unchanged. That sweep still has only 43/65 exact pixel results; its
clone, flex, vertical and flow-root cases remain open.

## Remaining constrained-column behavior

An additional eight-case scale-1 sweep limits the columns themselves to
80 pixels. Five match geometry and pixels; the 90, 200 and 300-pixel child
cases remain different. The original child-flow stream consumes the oversized
atomic unit, while the parent's own 160-pixel box must continue through two
80-pixel fragments. The remaining layout correction must preserve those
independent source intervals and the child border that finishes its own box.
The native geometry query exposes this remaining layout defect too.
The owning subsystem is `openui-layout` multicolumn continuation ownership,
with decoration replay in `openui-paint`; it is not a JavaScript requirement.

The later [atomic-child deferral candidate](native-atomic-column-deferral.md)
repairs those reduced capped states and verifies neighboring column heights
and nested padding. Its measurements have their own source and artifact
hashes; this checkpoint's earlier results remain preserved.

## Verification and limits

The candidate passes all 8,491 locked workspace tests, with 13 ignored,
including the 55 public Rust application scenarios. All 55 also pass with
Linux enabled after rebuilding the default-feature layout artifact. The C
ABI gate preserves 109 exports and the existing layouts and passes six C
consumers and the C++ header consumer. Read-only generators and formatting
pass. The completed candidate diagnostics have 640/640 focused and 960/960
primitive comparisons exact. The 1,920-case column cohort at four profiles
has 7,034/7,680 exact, 646 different and zero errors; all its Open UI RGBA
images are unchanged from the paint-only candidate. Compared with the clean
`822e0462` census, four already-different `block-max-height-004` comparisons
increase from 554 to 626 pixels at 1.25 and from 456 to 531 pixels at 1.5.
There are no exact regressions, and every Chromium RGBA image and oracle
identity is unchanged. Those remaining differences still need reviewed
ownership. These runs use dirty development source and do not qualify a
release. Complete clean renderer qualification remains required.

A local example build reused a paint-only layout artifact after a source
restore preserved an older file modification time. Its unconstrained
90-pixel query returned 68.671875 instead of the proven 93-pixel column height.
That artifact and its outputs are preserved as rejected evidence. Rebuilding
the production sources restores the correct result and reproduces all 40
correct images and geometry records byte for byte. The rejected artifact is
not used for renderer qualification.

These reduced measurements do not qualify the original 22,924 comparisons,
the complete expanded manifest, the final focused/primitive matrices, all
native APIs, or release hardware. Wider candidate checks remain separate
from the clean `547c7081` census. The complete original and expanded runs at
that preceding checkpoint are now indexed in the
[current status](../progress/current-status.md), with all renderer images
unchanged and the original remaining failures open.

## Reproduce

Build the native probe with the pinned configuration:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml build --locked \
  -p openui-engine --example constrained_box_geometry
```

Write each output into a new directory:

```sh
target/debug/examples/constrained_box_geometry \
  ../../out/native-atomic-new/height90 clipped-descendant 1.25 90
# The final optional argument constrains the column container itself.
target/debug/examples/constrained_box_geometry \
  ../../out/native-atomic-new/max80-height90 clipped-descendant 1 90 80
```

The preserved static HTML inputs are consumed by the separate Chromium
oracle. The Open UI probe performs every mutation through native Rust.
