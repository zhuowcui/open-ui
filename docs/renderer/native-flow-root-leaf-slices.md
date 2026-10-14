# Native flow-root child continuations

Chromium is the separate geometry and pixel reference. Open UI uses native
Rust document operations and Rust callbacks and executes no JavaScript.
Earlier inputs, captures and results remain immutable.

## Reviewed cause

An ordinary block's empty, fragmentable child already receives a retained
continuation for each column source slice. The corresponding preparation
was omitted for a `flow-root` container. Its child was replayed whole behind
each column clip, so the public geometry query exposed three 206-pixel boxes
at y=0, -68.671875 and -137.34375. Chromium exposes three actual child
continuations. The whole-box replay also selected the wrong border primitive
at fractional scales.

The shared multicolumn distribution now prepares ordinary child continuations
for both block and flow-root containers. The existing preparation retains
its exclusions for atomic children, nested columns, positioned content,
floats, transforms, avoided breaks and cloned decoration. Geometry, input
and painting use the same resulting fragments. The owner is `openui-layout`
multicolumn fragmentation. No fixture branch or output pixel correction is
added.

## Native measurements

The [diagnostic index](../v02/generated/native-flow-root-leaf-slices-v1.json)
records source, artifact and result hashes. At scales 1, 1.25, 1.5, 2 and 3,
all five reduced flow-root states match
Chromium's complete three-node geometry and pixels. The former fractional
differences of 222 pixels at 1.25 and 266 pixels at 1.5 are now zero.
The containing box keeps its own 160-pixel limit and the child's three
fragments retain the full 206-pixel source size.

The complete reduced sweep now has **55/65 pixels exact** and **50/65 complete
geometry exact**. All 60 other images and complete geometry records are
unchanged from the vertical maximum-size checkpoint. Cloned decoration,
constrained flex continuation and entirely empty bounds remain open.

## Public native Rust application

The [application consumer](../../bindings/rust/openui/examples/native_flow_root_geometry.rs)
imports only the public `openui` crate. It constructs the retained document
through `Document` and `Element`, applies typed styles, queries the owned
fragment rectangles, registers a Rust click callback, hits the overflowing
child in the last column and activates it through the native API. The
callback runs exactly once. It then exports geometry and a PNG through
public framework methods.

All five application outputs match Chromium's complete geometry and pixels.
Its geometry guard rejects the preceding renderer. Internal Engine probes
are retained as reduced diagnostics; this consuming Rust application verifies
the public API path needed by an app.

Run the consumer into a new directory:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml run --locked -p openui \
  --example native_flow_root_geometry -- ../../out/native-flow-root-new 1.25
```

The static HTML in `docs/v02/evidence/native-constrained-box-v1/flow-root`
belongs to the separate Chromium oracle. The native application loads no
HTML, scripts or browser runtime.

The locked workspace passes all 8,491 tests, with 13 ignored across 147
suites. The C geometry consumer performs the flow-root mutation through the
existing ABI and rejects the preceding library. All 109 exports and prior
layouts are preserved; six C consumers and the C++ header consumer pass.
The separate Linux Rust suite passes all 55 tests. Formatting and read-only
generators pass.

## Wider renderer checks

The candidate's focused and primitive 40-profile matrices are **640/640**
and **960/960** exact, with zero errors. The 1,920-case column cohort at four
profiles is **7,034/7,680** exact, with 646 differences and zero errors.
All 9,280 Open UI images, Chromium images and Chromium oracle identities
are unchanged from the preceding atomic-child candidate. The raw report
hashes and comparison audit are recorded in the diagnostic index.

These runs use an uncommitted candidate and have
`release_qualification: false`. Complete clean renderer matrices, closure
and reviewed ownership of the remaining differences, wider native API
coverage and release-lab gates remain required. No final renderer or
release qualification is claimed.
