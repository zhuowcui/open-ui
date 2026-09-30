# Native empty block border continuations

Chromium 147 is the pixel target. The
[diagnostic index](generated/native-border-fragment-diagnostic-v1.json) records
native Engine examples, preserved Chromium captures, exact pixel differences,
source and binary identities, and rejected experiments. It does not update the
complete census or qualify a release.

## Cause and repair

An empty bordered block inside a fragmented ordinary block container was
copied in full into every column. Each copy retained its top and bottom
border flags, and painting relied on the ancestor's clip to expose a slice.
Chromium instead gives the child its own continuations, suppressing border
sides at the column breaks. In pinned Blink's `BoxBorderPainter`, the single
stroked rectangle fast path requires all four visible sides. Suppressed sides
take a different paint path, with different fractional edge coverage.

The shared block layout step now materializes each empty child's source
interval, local geometry, first/last flags, and fragmentation direction.
Cloned decorations retain their established path. Flex and grid item geometry
remains with their respective fragmentation algorithms. The paint and raster
code is unchanged; there is no border-width threshold or test-ID branch.

At 1.25×, the reduced three-column example's inner border edge stored
`(64,64,0,255)` in Open UI and `(63,63,0,255)` in Chromium. Correct child
continuations select the appropriate existing border primitive and make this
example exact, reducing 222 differing pixels to zero. An ordinary border and
the same border under an ordinary clip were already exact.

## Evidence and limits

The native sweep uses five border widths, four offset phases, and device
scales 1, 1.25, 1.5, 2, and 3, at a 320×300 logical viewport:

| Context | Before exact | After exact | Comparisons |
|---|---:|---:|---:|
| Ordinary hollow border | 100 | 100 | 100 |
| Ordinary overflow clip | 100 | 100 | 100 |
| Fragmented block | 16 | 23 | 100 |

All 300 current native renders were rechecked against the preserved Chromium
captures. There were zero errors and no formerly exact reduced comparison
regressed. Thirty-one fragmented comparisons improved and seven already
failing comparisons worsened. Their remaining geometry and phase differences
are still open; this is not an exact primitive qualification matrix.

The final scoped diagnostic selection covers 28 original IDs at all four
required profiles: 75/112 exact, 37 different, and zero errors. Eleven
comparisons became exact, none regressed from exact, and every Chromium image
and oracle identity stayed fixed. One already failing comparison,
`out-of-flow-in-multicolumn-063` at 1.25×, increased from 62 to 63 differing
pixels.

The first complete clean run at `e51d88fd` was
[21,278/22,924 exact](generated/four-profile-census-v42.json), with 1,646
differences and zero errors. Its
[full delta](generated/native-empty-block-census-delta-v1.json) records 15 new
exact comparisons and three previously exact regressions in one nested
multicol case. No Chromium image or oracle identity changed. This checkpoint
is not accepted as a renderer qualification. The complete expanded matrix
has 22,081/23,728 exact, 1,647 differences, and zero errors; its 201 additions
remain 200/201 exact at every profile. The clean focused and primitive matrices
remain 640/640 and 960/960 exact.

The nested case contains an extracted spanner. Its source flow is split into
separate column rows, which own their continuation clips and overflow.
The ordinary ancestor's visual height is not a child source boundary. The
repair now checks the authored source owner and leaves those rows with the
spanner continuation path, including anonymous overflow pieces. A real layout
regression retains the leaf's source geometry and overflow in its owned row.
The [29-ID, four-profile diagnostic guard](generated/native-spanner-leaf-guard-v1.json)
is 79/116 exact, 37 different, and
zero errors: 12 new exact comparisons, no formerly exact regression, and
unchanged Chromium inputs. A new complete clean run is still required before
this corrected source replaces the earlier census evidence.

An earlier candidate regressed six exact cloned-decoration comparisons and
was rejected. A second had no exact regressions across the complete
4,880-comparison fragmentation diagnostic but failed two existing flex
geometry tests. Restricting this
step to ordinary block containers preserves those tests without changing
their assertions. The scoped implementation passes all 8,481 locked workspace
tests, with 13 ignored. The layout regression verifies the child's actual
three source intervals and first/last flags through the real layout entry
point. The earlier candidate's 640 focused and 960 primitive comparisons were
exact and all images matched the previous clean guards; these dirty runs do
not replace clean qualification evidence.

The first reduced HTML input accidentally gave its auto-height body an extra
overflow clip. Those inputs and captures remain preserved as diagnostics.
The versioned input used here fixes the body's viewport height to match the
native setup; no original WPT fixture or cached oracle was edited. Nine fresh
capture attempts in the final sweep required another attempt. Failed attempts
remain preserved, and the final accepted captures completed without errors.

## Reproduce

Build `openui-engine`'s `border_raster_reproducer` example with the checked-in
Chromium Cargo configuration. It constructs the state using public typed
Engine methods and writes a native PNG and equivalent static HTML. Open UI
executes no JavaScript. The HTML is input to the separate Chromium oracle.

```sh
cd bindings/rust
CARGO_INCREMENTAL=0 cargo --config .cargo/config.chromium.toml build --locked \
  -p openui-engine --example border_raster_reproducer
target/debug/examples/border_raster_reproducer \
  /tmp/openui-native-border-columns 1.25 3 0 320 300 columns
```

`tools/qualification/probe_border_fragments.py` runs the complete context sweep
with explicit baseline and candidate example binaries. Every run requires a
new result directory and preserves fresh Chromium captures. The three
[reduced inputs](reproducers/native-border-fragments-v1/columns/test.html) and
their PNGs are committed for review. Historical Open UI outputs remain
provenance; exact comparison always uses Chromium pixels.
