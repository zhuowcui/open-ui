# Native bounds for entirely empty fragments

Chromium is the geometry and pixel reference. Apps query their retained
document through native Rust methods; Open UI executes no JavaScript.

## Reviewed cause and correction

Open UI returned the first rectangle when every retained fragment was
empty. Chromium's bounding-rectangle implementation unions fragment
rectangles in layout order. An empty accumulator is replaced by the next
rectangle, including another empty rectangle. When every fragment is
empty, the resulting bounds therefore retain the final rectangle.

The shared `Engine::node_bounds` fallback now uses the final rectangle.
Nonempty rectangles retain the existing union; empty rectangles do not
enlarge it. Elements without a layout box still return `None`. Rust
geometry queries, C queries, accessibility bounds and view timelines use
that same retained geometry. Returned snapshots remain owned by callers.
There is no fixture branch, layout change or raster correction.
The owner is `openui-engine` retained geometry collection.

## Reference and native verification

The immutable five-scale `max0` Chromium measurements are in the earlier
constrained-box evidence. They give three owned rectangles at x=0, 108 and
216, each 84 pixels wide and zero pixels high. The combined bounds are
**(216, 0, 84, 0)**. The source review records Chromium's element query and
rectangle-union implementation separately from the pinned capture binary.

The public Rust application scenario now checks all three owned rectangles,
the combined bounds, accessibility bounds, and earlier snapshots after
mutation. The C consumer performs the same zero-height mutation through
the existing ABI. Both consuming guards fail against the preceding shared
engine and pass with this correction.

The locked workspace passes **8,491 tests**, with 13 ignored across 147
suites. Formatting and read-only generators pass.
All **55 Linux application scenarios** pass. The C ABI preserves all
**109 exports** and prior layouts; six C consumers and the C++ header
consumer pass. The complete reduced sweep has **55/65 geometry exact** and
**55/65 pixels exact**. All five entirely empty bounds are newly correct,
all 60 other complete geometry records are unchanged, and every one of
the 65 PNGs is byte-identical to the preceding flow-root output.

The [diagnostic index](../v02/generated/native-empty-fragment-bounds-v1.json)
records source, binary, comparison and verification hashes. The Chromium
inputs, archived renders and earlier evidence remain unchanged.

These development measurements have `release_qualification: false`.
Complete clean renderer matrices, remaining native behavior and release
gates remain required. This API correction does not qualify the renderer
or the final release.
