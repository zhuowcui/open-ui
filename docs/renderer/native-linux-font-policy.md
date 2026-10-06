# Native Linux font scale policy

Open UI never executes JavaScript. Native apps use public Rust methods and Rust
callbacks. Pinned Chromium supplies the expected pixels and element bounds.

## What was wrong

The private Fontations outline trial matched 62/200 native app images. Its Linux
font policy treated Skia's positioning flag and the platform's subpixel policy
as the same setting. These flags have different effects in Chromium.

The reviewed Chromium source enables platform subpixel positioning above scale
1 and disables hinting there. Skia may separately force its positioning flag at
unit scale. Platform positioning selects linear metrics and the adjustment that
borrows a pixel from rounded ascent when rounded descent would clip a glyph.
This rule depends on rendering settings and scale, not a font name or test ID.

The independent `native-font-scale-query-v1861` observations cover Ahem and
DejaVu Sans at 10px, 12px and 16px, over five scales. Each query repeats in an
independent Chromium process. The old native metrics differ in 12/30 cases.
The shared correction matches all 30. Identical guards reproduce the old
failure and pass on the candidate; the first candidate passes 344 text tests.

## Preserve the caller's choice

The first correction also changed an explicitly requested slight hinting mode
above scale 1. That is an API defect. A separate guard reproduces it before
the correction.

The revised private Rust API adds `TextHinting::ChromiumLinux` and
`TextRasterConfiguration::chromium_linux_platform`. The explicit Fontations
constructor selects that policy before document construction. Fixed `None`,
`Slight`, `Normal` and `Full` requests keep their requested fitting mode at
every tested scale. The original retained configuration stays immutable;
`resolved_hinting` derives the scale's effective fitting mode. Emphasis painting
also resolves that mode. The candidate passes all 345 text tests, including
both the independent metrics guard and the fixed-request guard.

This API is private trial code. It is not part of the integrated public SDK,
and complete C parity and typeface behavior remain required.

## Native app evidence

The existing Rust app creates 64 text positions, draws `X` in black, then uses
a Rust click callback to change it to `XX` in blue. It checks retained options,
owned element bounds, callback count and teardown. Four font families, five
sizes and five scales produce 200 before/after images. Every native process
repeats. The Chromium images and their repeated capture/query records are
reused unchanged.

The first corrected trial finishes at **162/200 exact**, gaining 100 exact
images and losing none against the previous 62/200 Fontations trial. It retains
all 40 exact images at scale 1. There are 38 remaining image differences.
Element geometry remains **11,520/12,800 exact**, unchanged from that trial.
Its actual pixel gate exits 1. Passing font metrics does not establish exact
element bounds or pixels.

The revised automatic-policy source completes the same whole sweep: 162/200
images and 11,520/12,800 bounds are exact. All 200 native PNGs are byte-identical
to the first correction, every Chromium image is unchanged, and no exact
comparison is gained or lost by the API correction. Both sweeps finish with
exit 1. Four hundred independent native processes verify repeated output,
callbacks and teardown across the two candidates.

| Scale | Exact images | Total |
|---|---:|---:|
| 1 | 40 | 40 |
| 1.25 | 26 | 40 |
| 1.5 | 36 | 40 |
| 2 | 38 | 40 |
| 3 | 22 | 40 |

The [preserved evidence](generated/native-linux-font-policy-v1.json) records
both baseline failures, both completed builds and image sweeps, the failed
disk preflight, all reference identities and the complete paired comparison.
The archive includes the exact native images, source snapshots and the
[reviewable implementation patch](evidence/native-linux-font-policy-v1/shared-linux-policy.patch).
Every archived member is verified by hash.

A [complete reproduction patch](evidence/native-linux-font-policy-v1/reproduction-from-16187f4f.patch)
applies to public checkpoint `16187f4f`. Applying it to an independent Git
index reconstructs the candidate's entire tracked source tree exactly,
including its Rust app and guards. The
[source reconstruction record](generated/native-linux-font-policy-source-v1.json)
preserves both tree IDs and the patch hash. This check performs no new renders
and does not relabel the measured private commit.

For example, Ahem at 12px and 16px has exact unit-scale image pixels while its
element width still differs: native widths are 12 and 16; Chromium's widths are
12.015625 and 16.015625. Font matching and shaping still use FreeType. The
Fontations outline adapter is not a complete Fontations typeface factory.

The pinned Skia source exposes another concrete difference to investigate.
`SkTypeface_Fontations::onFilterRec` clears `kGenA8FromLCD_Flag`;
`SkUserTypeface::onFilterRec`, used by the current outline builder, does not.
That changes the mechanism used to generate grayscale glyph coverage. It is
a candidate explanation for the remaining large-strike differences, not a
proved cause for every residual. The next factory work must preserve these
shared scaler decisions and native advances, rather than compensate by font
name, size, phase cell or output pixels.

## Remaining qualification

The [real Fontations factory prototype](native-fontations-factory.md) now
builds and passes standalone probes using the unchanged Skia pin. Its
[current-branch native app trial](native-fontations-app.md) matches all 200
Chromium images, gaining the remaining 38 with no losses. Bounds still fail
in 1,280 states. These newer results do not relabel the outline trial below;
full integration and qualification remain required.

These sources inherit the previously rejected descriptor correction, which
lost 29 exact full-census comparisons. They also predate the integrated variable
font instance correction. Neither change may silently replace the accepted
renderer. Full original, expanded, focused and primitive matrices have not run
on these new sources. Full workspace tests and C/C++ ABI consumers have not
run on them either. No WPT state is admitted and no new formal original-renderer
residual ownership is assigned.

The shared Linux policy must be combined with the current implementation,
complete native typeface/advance behavior, fixed-request semantics and C
interfaces, then pass the unchanged Chromium gates. Default native text,
neighboring fonts, writing modes, effects and all remaining element APIs still
need qualification.

The reviewed source checkout is Chromium 147.0.7727.24; the measured pinned
binary is 147.0.7727.50. Source review is not a runtime typeface-factory trace.
The actual feature parameter spelling is `Fontations`; the source defaults to
that choice on Linux. A binary dispatch trace is still absent.

The separate umbrella checkpoint `0c39998c` passes all four hosted workflows
and all seven full hardening jobs. These jobs qualify that checkpoint's measured
non-pixel checks, not these private sources or the open full Chromium gates.

The automatic-policy retry first stops at the disk guard during workspace
cleaning, before any tests or raster runs. That failed source and receipt stay
immutable. Three completed older build binaries move to the data drive with
all hashes and original paths preserved. Fresh retry roots and result paths
preserve the failed attempt and all reference bytes.
