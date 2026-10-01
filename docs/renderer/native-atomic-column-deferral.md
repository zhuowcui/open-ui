# Native atomic children across column boundaries

Open UI constructs and mutates these states with native Rust APIs and runs no
JavaScript. Chromium 147.0.7727.50 supplies the separate geometry and pixel
reference. Every earlier fixture, image, manifest and result remains preserved.

## Reviewed cause and correction

An atomic child can fit the balanced column size but fail to fit after its
parent's leading border or padding when the column has a maximum height.
The layout previously consumed that child in the first column, losing the
legal break before it. The shared block-flow preparation now reserves the
remaining fragmentainer space and places the child in the next column. It
uses the accumulated source offset through nested padding and margins.

A definite containing block retains its own fragment before that child
starts. The child's later source progress is bounded by the containing box's
remaining used size; its atomic ink may overflow that box. Geometry and paint
consume the same retained fragments. This changes the shared layout data,
without selecting fixtures or changing raster output after painting.

The owner is `openui-layout` multicolumn fragmentation. The preceding
[painting and balancing checkpoint](native-monolithic-column-balancing.md)
remains preserved with its separate source hashes and measurements.

## Measured states

The [diagnostic index](../v02/generated/native-atomic-column-deferral-v1.json)
records **144/144** native states with exact Chromium geometry for all three
queried nodes and zero different pixels:

- eight atomic child heights at five scales with unconstrained columns;
- the same eight heights and five scales with columns capped at 80 pixels;
- the same capped states with ten pixels of nested leading padding;
- 24 neighboring states with column caps of 50, 85 and 120 pixels at scale 1.

The scales are 1, 1.25, 1.5, 2 and 3 at a 320 by 340 logical viewport.
Every Chromium input is static HTML. Each capture preserves two agreeing
geometry queries. The native probe creates every element and applies each
property through Rust Engine operations. Forty-eight static inputs and the
scale-1 Chromium geometry are checked in as versioned reduced evidence.

With an 80-pixel column cap, the 90-pixel child's bordered parent has fragment
heights 80, 90 and 36. The 200- and 300-pixel children produce parent fragments
of 80 and 126. Their outer 160-pixel box retains two 80-pixel fragments.
Adding ten pixels of leading padding produces bordered fragments of 70 and
136 for the 200-pixel child, starting at y=10 and y=0 respectively.

The earlier 13-case, five-scale constrained-box sweep retains all **65/65**
native PNG bytes and complete geometry records. Its existing clone, flex,
vertical and flow-root failures remain open; 43/65 pixels match Chromium.

## Public application verification

The public Rust application consumer changes the child and column heights,
checks the owned fragment lists, renders the first border and background,
then adds nested padding and checks the updated geometry. All 55 application
scenarios pass with Linux enabled. The complete locked workspace passes
8,491 tests with 13 ignored across 147 suites.

The native C geometry consumer performs the same capped mutations through
the existing property and geometry exports. Its new assertions fail against
the preceding library and pass with this candidate. The ABI gate preserves
all 109 exports and struct layouts and passes six C consumers plus the C++
header consumer. Read-only generators and formatting pass.

The completed development matrices are **640/640 focused** and **960/960
primitive exact**, with zero differences or errors. The four-profile,
1,920-case column cohort is **7,034/7,680 exact**, with 646 differences and
zero errors. All 9,280 Open UI RGBA images, Chromium RGBA images and oracle
identities are unchanged from the preceding painting and balancing candidate.
No formerly exact comparison regresses. The diagnostic index records each
report, source identity and hash; the residual column failures remain open.

These development measurements have `release_qualification: false` and use
dirty source. Complete clean original, expanded, focused and primitive
matrices, reviewed residual ownership, remaining native API coverage and
release-lab gates remain required for the committed source. No release or
full renderer qualification is claimed by these results.

## Reproduce

Build the native probe using the pinned configuration:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml build --locked \
  -p openui-engine --example constrained_box_geometry
target/debug/examples/constrained_box_geometry \
  ../../out/native-atomic-new/max80-height200 clipped-descendant 1.25 200 80
target/debug/examples/constrained_box_geometry \
  ../../out/native-atomic-new/padded-height200 clipped-descendant 1.25 200 80 10
```

Each output directory must be new. The final optional argument applies the
wrapper's native `PaddingTop` property. The separate Chromium oracle consumes
the preserved static HTML inputs; Open UI never executes their test queries.
