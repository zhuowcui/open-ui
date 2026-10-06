# Native app with the real Fontations factory

Open UI never executes JavaScript. Applications use public native Rust methods
and Rust callbacks. Pinned Chromium supplies the expected pixels and element
bounds. Historical Open UI output remains immutable provenance.

## Completed result

Clean private source `9e4baa08` renders **200/200 native app images exactly like
Chromium**. It gains the remaining 38 exact images over the earlier 162/200
outline trial and loses none. All 12,800 measured logical phase cells have
exact pixels. The full original renderer census has not run on this source.

The Rust app creates 64 text positions, draws `X` in black, then uses a Rust
click callback to change the text to `XX` and the color to blue. Four font
families, five sizes and five scales produce 200 before/after images. Two
independent native runs check each combination: all 200 processes finish,
their PNG bytes repeat exactly, callbacks fire once, owned options and bounds
remain usable, and weak element handles expire after teardown.

Every Chromium image, font asset, input and repeated reference query remains
unchanged. This result uses the existing references; it creates no new oracle
bytes and does not replace a frozen reference.

| Scale | Exact images | Total |
|---|---:|---:|
| 1 | 40 | 40 |
| 1.25 | 40 | 40 |
| 1.5 | 40 | 40 |
| 2 | 40 | 40 |
| 3 | 40 | 40 |

## What changed

The [standalone factory prototype](native-fontations-factory.md) is combined
with the current branch's native font resolution. An explicit immutable
`RasterConfiguration::chromium_linux_fontations_lcd` selects the real
Fontations typeface and scaler from the existing Skia pin. System and
application registries still select the face. Its bytes, collection member,
variation and palette arguments reach the native factory; shaping and painting
then use that typeface.

The new path uses ordinary Skia text blobs. It bypasses the custom outline
adapter and the legacy FreeType glyph-origin compensation. It does not choose
an engine by font name, size, test ID or environment, and does not patch output
pixels. Unsupported factory data returns no face instead of silently switching
to FreeType. The existing default and explicit FreeType choices remain in the
source and require complete regression qualification.

The candidate passes all **344 text tests** and **16 read-only source checks**.
Its metrics guard verifies a real `fnta` typeface and all 30 independent
Chromium metrics observations. Fixed hinting requests remain fixed at the five
tested scales. This source retains the integrated variable-font correction;
it does not inherit the old descriptor trial's 29 census losses.

## Geometry still fails

Element bounds remain **11,520/12,800 exact**. All 12,800 native bounds are
unchanged from the outline trial. The remaining 1,280 differences affect width
alone, each exactly 1/64 CSS pixel short. No difference is tolerated: the
complete native gate exits **1**, despite the exact image pixels.

The existing [intrinsic width investigation](native-intrinsic-snap.md) identifies
a shared helper that discards small positive shaped fractions before rounding
to the layout grid. Its earlier general correction closes native widths but
loses 21 exact original-renderer comparisons, so it remains rejected. The real
factory does not remove this allocation issue. Text and layout still need a
reviewed measurement fix and guards for the earlier losses; no font-specific
width adjustment is justified.

## Evidence and reproduction

The [versioned record](generated/native-fontations-factory-v2.json) preserves
the completed owner, native app sweep, full paired audit, reference identities,
source checks, source files and all failed attempts. Every member of the
[evidence archive](evidence/native-fontations-factory-v2/completed-evidence.tar.gz)
is hash-verified.

The [complete source patch](evidence/native-fontations-factory-v2/reproduction-from-9f0d2211.patch.gz)
decompresses to the original patch bytes and applies to public checkpoint `9f0d2211`. An independent Git index reconstructs
the candidate's entire tracked source tree exactly. The new crate vendors the
113 unchanged pinned headers needed by the factory and does not need the
prototype's external include-path variable. Source reconstruction is not a
new render or a relabelled source measurement.

The first current-branch build records its source before an unlocked Cargo
clean completes dependency resolution. Its 344 passing text tests and app
build therefore have dirty source identities. The native worker also fails
its owner-environment preflight before running an app. Its incomplete receipt
and source remain unchanged; a separate terminal observation records the
finished processes. A fresh retry uses the complete lockfile, a regenerated
dependency-policy artifact, locked cleaning, a clean commit and the correct
owner environment. All measured retry stages retain their clean source
identity. Earlier file-mode and header-declaration preflight failures are also
preserved and are not passes.

## Remaining qualification

This candidate is private and unapplied. It admits no release state or formal
original-census residual owner. Its original, expanded, focused and primitive
renderer matrices, full workspace tests, MSRV, Miri, C ABI consumers and
packaging have not run. Native C selection, collection and variation behavior,
palette effects, compressed fonts and every configuration field still need
behavior qualification. Default native rendering, other glyphs, writing modes,
effects and all remaining public element APIs stay required.

The accepted complete census remains 21,334/22,924 original and
22,137/23,728 expanded exact, zero errors and actual exits 1. These counts are
not results for the new source. Matching this native font corpus does not
declare full renderer or release qualification.

Separate umbrella checkpoint `412e804c` passes all three PR workflows: six
jobs pass, five skip and none fail. All seven native guards actually run and
pass in hosted parity. Full manual hardening has not run on that checkpoint.
Hosted results do not waive the candidate's geometry or other open gates.
