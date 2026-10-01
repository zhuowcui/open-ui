# Native constrained column-flex continuations

Chromium is the separate geometry and pixel reference. The consuming app
constructs its document and handles input through public Rust methods and
Rust callbacks. Open UI executes no JavaScript.

## Shared layout correction

A maximum-height column flexbox owns a border box independently of the
source space occupied by its flex items. The multicolumn measurement had
treated this item overflow as an indivisible visual extent. It chose a
206-pixel column for a fragmentable item instead of balancing its source
into three columns. Later distribution could also give the containing box
the item's continuation size and replay the whole item behind each clip.

The correction uses the existing forward column-flex item source extent
in initial measurement, repeated measurement and distribution. The
containing box retains its own rectangle and decoration budget. Atomic
children and avoided breaks supply their minimum balancing unit. Padding
belongs to the source slice once. Ordinary fragmentable leaf items retain
actual continuations, and the final item's source end is preserved instead
of growing it to fill a rounded column interval. Row and reversed flex
retain their existing paths and remain unfinished where measured below.

The owner is `openui-layout` multicolumn fragmentation. This correction
changes retained fragments consumed by geometry, input and painting; it
adds no test-ID branch, pixel correction or tolerance.

## Native app measurements

The [public Rust consumer](../../bindings/rust/openui/examples/native_column_flex_geometry.rs)
imports only `openui`. It uses typed style methods, queries owned fragments
and combined bounds, and exports its PNG through the framework. In the
default `max160` state it hits the last-column child and activates a Rust
click callback exactly once. The unchanged application guard rejects the
preceding renderer. The C consumer mutates the same retained engine and
checks fragments, owned copies and last-column input; its new guard rejects
the preceding flow-root library.

At scales 1, 1.25, 1.5, 2 and 3:

| Native case | Complete geometry | Exact pixels |
|---|---:|---:|
| Maximum height 160 | 5/5 | 5/5 |
| Maximum height 120 | 5/5 | 5/5 |
| Padding at the block start | 5/5 | 5/5 |
| Clipped child, height 20 | 5/5 | 5/5 |
| Clipped child, height 90 | 5/5 | 5/5 |
| A following flex item | 5/5 | 5/5 |
| Avoided item break | 5/5 | 5/5 |
| Following normal block | 5/5 | 0/5 |
| Zero maximum height | 0/5 | 5/5 |
| Reversed column flex | 0/5 | 3/5 |
| Row flex | 0/5 | 0/5 |

That is **40/55 complete geometry exact**, **43/55 pixels exact**, and
seven case variants meeting both gates at all five scales. No previously
exact native geometry or pixel comparison regresses. Thirty comparisons
become exact in both geometry and pixels.

The earlier 65-case native sweep is now **60/65 pixels exact** and
**55/65 complete geometry exact**. All 60 other images and complete geometry
records are unchanged. All **144** atomic-child neighbor states remain
geometry- and pixel-exact, with complete geometry and PNG bytes unchanged.
The [diagnostic index](../v02/generated/native-column-flex-overflow-v1.json)
records source and binary hashes, separate Chromium inputs and captures,
comparison hashes and the remaining failures.

The locked workspace passes **8,491 tests**, with 13 ignored in 147 suites.
All **78** public Rust Linux unit tests and the separate **55** Linux
application scenarios pass.
The C ABI retains all **109 exports** and prior layouts; six C consumers and
the C++ header consumer pass.

Run a native state into a new directory:

```sh
cd bindings/rust
cargo --config .cargo/config.chromium.toml run --locked -p openui \
  --example native_column_flex_geometry -- ../../out/native-flex-new 1.25 max160
```

Capture the separate static Chromium inputs into a new directory:

```sh
python3 tools/qualification/probe_native_geometry_oracle.py \
  --suite column-flex --scale 1.25 --results-dir out/chromium-native-flex-new
```

## Remaining native behavior

- **Following normal block:** geometry agrees, but its background erases
  the overflowing flex item's border. Chromium paints that border above
  the block background. The owner is `openui-paint` in-flow paint phases.
- **Zero maximum height:** the owned fragment list agrees, but entirely
  empty combined bounds select the first fragment; Chromium selects the
  final fragment in this reduced case. The owner is retained geometry
  collection in `openui-engine`/`openui-paint`.
- **Reversed column flex:** whole-item replay and final continuation growth
  still expose incorrect source rectangles and fractional border coverage.
  The owner is `openui-layout` reversed flex fragmentation.
- **Row flex:** the visual-overflow measurement still treats the item as
  indivisible. The owner is `openui-layout` row-flex multicolumn measurement.

These are needed native behaviors and remain implementation work. The
presence of a public method or a native fixture does not count its
incorrect result as complete.

These uncommitted development measurements have
`release_qualification: false`. Candidate raster matrices, wider column
checks, complete clean original and expanded matrices, remaining native API
behavior and release-lab gates remain required. No renderer or final release
qualification is claimed.

## Completed candidate matrices

The broader four-profile 1,920-case selection is **7,034/7,680 exact**, with
646 differences and zero errors. Focused **640/640** and primitive **960/960**
matrices are exact. Every one of these 9,280 Open UI images, Chromium images
and oracle identities is unchanged from the preceding flow-root candidate.
All three reports record the same development source and explicit CPU backend.
These completed candidate checks are diagnostic; complete clean matrices are
still required. The zero-height bounds result above precedes the separate
[native empty-bounds correction](native-empty-fragment-bounds.md).

## Combined native checkpoint

The column-flex correction and the separate empty-bounds correction are now
verified together. The [v2 index](../v02/generated/native-column-flex-overflow-v2.json)
records **60/65 complete geometry and pixels exact** in the original reduced
sweep, **45/55 geometry exact**, **43/55 pixels exact**, and **40/55 exact in
both** for the public Rust app. The zero-height case now matches all five
Chromium measurements. Every PNG byte is unchanged from the column-flex v1
candidate, no exact native geometry comparison regresses, and all 144 atomic
guards remain geometry- and pixel-exact. Cloned decoration, row/reversed flex
and following-block paint order remain open.

The combined locked workspace passes 8,491 tests with 13 ignored in 147
suites. All 55 Linux application scenarios pass. Six C consumers and the
C++ consumer pass with all 109 exports and prior layouts intact. The new
C fragment guard rejects the preceding empty-bounds library at an assertion.
Complete clean renderer matrices remain required.
