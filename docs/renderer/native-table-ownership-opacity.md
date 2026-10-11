# Native table ownership and opacity

Open UI is a native framework. Apps create and change elements through public
Rust methods and callbacks over the shared Engine. Open UI runs no JavaScript.
Pinned Chromium 147.0.7727.50 supplies expected pixels and geometry. Earlier
Open UI images are preserved evidence of past results.

## Table ownership: what is verified

Clean `80797e94` was measured by owner 4182 and independently checked by audit
4183. The public native consumer uses five states at scales 1, 1.25, 1.5, 2
and 3. The states are base, translated, scrolled, clipped and opacity.

| Check | Exact result | Scope |
|---|---:|---|
| Pictures | 20/25 | Five opacity pictures differ |
| Named geometry records | 275/275 | Bounds and ordered client rectangles |
| Hit checks | 250/250 | Ten points in each state/profile |
| Existing scoped tests | 736 passed | 675 layout, two ownership paint, 59 native conformance |

The repair gives authored boxes and table formatting helpers distinct layout
roles. Helpers retain their style source, decoration and structural clipping.
The owning element supplies its authored transforms, opacity, clips, scrolling
and input identity. Real inline and fragmented continuations retain their
separate rectangles.

The measured owner exited 1 because pixels still differed; the independent
audit exited 0 after checking the complete result. Both were reaped before
later source or target changes. This verifies the reduced geometry and input
cases. Full native API coverage, original table glyph regressions and renderer
qualification remain open. The prior collapsed-border corner hypothesis still
needs its declared source-to-operation proof and guards.

The older [table-origin diagnostic](native-table-origin.md) remains historical
evidence from clean `c6a0b92e`. Its 8/20 pictures and 75/170 geometry results
must stay attributed to that measurement.

## Opacity: what the CPU test establishes

Owner 4199 measured 720 operation cases twice at clean `80797e94`; audit 4200
checked the completed result. Both runs produced identical output bytes. The
test uses the same Skia package, features and compiled libraries as owner 4182.
It includes production sRGB and no-color-space surfaces, five scales, three
colors, N32/F16/folded paint, single and nested opacity, one or two draws, and
direct versus recorded playback. Each owner and audit exited 0 and was reaped.

N32 is the ordinary 8-bit intermediate. F16 stores channels as 16-bit floating
point values. Their flat interior results differ in the measured playback
paths:

| Recorded single 0.5 opacity group | Blue RGBA | Green RGBA |
|---|---|---|
| Multiple draws, N32 | `(126,126,255,255)` | `(126,191,126,255)` |
| Multiple draws, F16 | `(128,128,255,255)` | `(128,191,128,255)` |
| One draw or explicitly folded paint | `(127,127,255,255)` | `(127,191,127,255)` |

The Chromium table captures have the first row's blue and green interiors.
Native vector-only groups had selected F16 and produced the second row.
Nested 0.5/0.5 red gives `(255,191,191,255)` in the measured paths. That nested
agreement does not establish general single-group or overlap equality.

The pinned source review preserves Chromium's ordinary SaveLayerAlpha path,
its eligible single-draw opacity folding, and the corresponding Skia source.
Chromium's exact DEPS pins Skia `abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0`.
The CPU measurement reproduces the format and recording distinction; it does
not trace every Chromium draw or qualify the current Engine.

## Shared policy repair and measured Engine result

At clean `c4e8a809`, all authored opacity groups use the destination's ordinary
intermediate format. Formatting helpers still do not apply authored effects.
Clip, filter and mask order, disabled opacity folding and the separate F16
circular-background primitive remain unchanged.

Owner 4202 rebuilt all sixteen local packages. Audit 4203 independently checked
all repeated native and Chromium images and snapshots before process reaping
was saved.

| Check | Exact result |
|---|---:|
| Pictures: five states at five scales | 25/25 |
| Full node geometry records | 275/275 |
| Hit checks | 250/250 |
| Scroll snapshots | 25/25 |
| Layout tests | 675/675 |
| Native conformance tests | 59/59 |

Ninety native processes and ten Chromium processes completed. Rust callback,
unchanged-frame and weak-handle teardown checks pass. The twenty older table
comparisons add zero newly wrong pixels against unchanged Chromium references.
They remain 8/20 pictures and 85/170 geometry records exact, so original glyph
and collapsed used-border repairs remain open.

The full paint suite initially passed 168 tests and failed two outline guards.
Those guards supplied `NodeId::NONE`, representing no authored box under the
ownership contract. Their setup now creates real document nodes; all exact
pixel assertions are unchanged. At clean `0d178e2c`, owner 4208 and audit 4209
verify **170/170 paint tests passed**, with no ignored or filtered tests. The
production painter is byte-identical to the measured `c4e8a809` production
code. The failed run remains preserved with its actual owner exit 1.

The common policy fixes the measured table opacity difference without per-test
colors, alpha offsets, rewritten references or pixel tolerance. It still needs
the focused and primitive matrices, both complete censuses, and exact combined
clip/filter/mask/promoted-transform guards before global qualification.

The last complete original census remains **21,329/22,924 exact (93.04%)**, with
1,595 differences and zero render errors. No new complete original or expanded
count is available. The percentage describes comparison results. Completion
of all renderer, native API, compositor, hardware and release work is unmeasured.

## Evidence

The [immutable evidence index](generated/native-table-opacity-v1.json) records
source identities, original receipts, compiler hashes, native/Chromium PNG
bytes, full geometry, channel deltas, repeated runs, callbacks and actual
process reaping. Its 1,154 members retain two incomplete diagnostic attempts,
the earlier consumer build failure and the failed full paint run. These failures
are not passes. Every stored and decompressed member hash was verified.

Primary source receipts pin Chromium 147.0.7727.50. Historical Open UI bytes
support investigation and regression analysis; Chromium alone supplies
expected pixels and geometry. Open UI runs no JavaScript in any version.
Any needed browser-style element operation must be exposed as a native Rust
API over the retained Engine.
