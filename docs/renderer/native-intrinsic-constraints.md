# Native intrinsic constraints and leading whitespace

Open UI never executes JavaScript. Applications create and change elements
through public Rust methods and Rust callbacks. Needed browser element behavior
must be implemented in Rust and exposed to the consuming native app. Chromium
is the sole pixel target, with zero tolerance.

## Completed fieldset trial

Clean private `abed078d` finishes both complete censuses:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 21,270 / 22,924 | 1,654 | 0 | 1 |
| Expanded | 22,069 / 23,728 | 1,659 | 0 | 1 |

The [complete audits](generated/native-scroll-insets-v37.json) retain the
Chromium image hashes and oracle identities. Against its intrinsic-sizing
parent, the fieldset repair restores six exact comparisons and improves two
more. Against the accepted renderer, it still loses 64 original exact
comparisons and four addition comparisons, with no exact gains. Only 199/201
addition cases are exact at all four profiles. All original rows agree between
the two suites. The source remains unapplied; the accepted totals stay
21,334/22,924 original and 22,137/23,728 expanded exact.

The generated ownership ledger rejects unowned residuals. Assigning a broad
layout owner in a diagnostic audit does not complete minimized cause review.
Neither complete census qualifies this trial.

## Two shared layout causes

The investigation queries six cases using two already built native runners.
There are 12 default-profile observations and 48 observations with the actual
census viewport, scale and immutable raster configuration. All commands exit
zero. These commands print Engine geometry and record scenes; they do not
build Rust code or rasterize images.

| Cause | Measured native result | Chromium result | Prepared correction |
|---|---|---|---|
| Atomic inline contribution drops the child's minimum width | Wrapper width 42 px; principal inner box remains 200 px | Wrapper 202 px | Reuse the child contribution already resolved by shared layout, including preferred/min/max constraints |
| Flattened intrinsic scan counts collapsible leading whitespace | Table-caption float 130 px; line-clamp float 112 px in the aliased census profile | 114 px and 96 px | Track whether the line contains content separately from indentation and inline decoration widths |

`openui-layout` owns these causes. Private `7d723caa` prepares the shared
corrections without a fixture-ID or post-raster branch. Direct children reuse
their existing contribution map; nested atomic items call the same shared
contribution function. The whitespace scan preserves cumulative advances and
collapses leading space before creating an interior break.

Other intrinsic cases, including orthogonal percentage sizing and clearance,
remain unresolved. Correct bounds alone do not establish exact pixels.

## Font context measurements

Two independent Chromium processes query 100 text variants at each of five
scales: 1,000 observations. Element geometry agrees between repeats and across
scales. These queries generate no screenshots or Canvas pixels.

The installed Ahem file and the app-owned font resource have identical bytes.
Chromium nevertheless measures five `X` characters at 16 px differently:

| Font context | Canvas advance | Logical box width |
|---|---:|---:|
| Installed font | 80 | 80 |
| Same bytes loaded as a font resource | 80.00015258789062 | 80.015625 |

Leading and trailing collapsible whitespace leaves either width unchanged.
The existing immutable raster configuration distinguishes the tested native
profiles. These measurements do not authorize font-name-specific rounding,
a tolerance, or replacement of any reference image.

## Consuming Rust app and pending qualification

The [native app](evidence/native-intrinsic-constraints-v1/native_intrinsic_constraints.rs)
uses public typed methods to create the document, selects its raster policy
before creation, and changes minimum/maximum width or text/indentation from a
Rust click callback. It checks owned bounds and handle teardown. The
[source patch](evidence/native-intrinsic-constraints-v1/native-intrinsic-constraints.patch)
includes named Engine guards and the applied owned-recording cache code.
Compiled workspace source attribution still requires the full clean rebuild.

The atomic baseline reproduces its named assertion failure. The leading-space
baseline instead fails to compile: `Float` has no implicit conversion to
`StyleValue`. The [v40 correction](generated/native-scroll-insets-v40.json)
uses the existing `RendererStyleValue::Float` variant at fixed `a6d386e4` and
test-only baseline `ac1eb2a7`, preserving all assertions and production code.
Ten read-only checks pass on the corrected fixed source. Its remaining named
baseline failures, fixed guards, consuming-app execution and exact pixels
remain pending. The
source inherits an unqualified intrinsic trial, so earlier focused/primitive
passes do not qualify this new source.

The new local pipeline cleans all 18 workspace packages at every source
switch. It checks 1,000 prior native sizing observations, 400 native callback
images, 280 selected comparisons, both complete 40-profile matrices and both
complete censuses. Each new Chromium state requires two consecutive captures
in each of two independent processes; unequal captures are preserved without
retry or tolerance. Every preceding whole pipeline must finish before this
one starts. Local builds and image sweeps remain separate.

The first waiting pipeline was stopped before any stage ran and replaced with
that stronger build isolation. Two prior font/C builds reported missing
geometry members that are present in their clean source; stale workspace
artifacts are suspected and require a clean rebuild. Separate raster guard
probes also contain invalid test-only `fields_mut` calls. Compilation failure
does not reproduce a named regression or establish an API pass.

No new release state is admitted. Missing needed public Rust operations remain
unfinished API work, including for cases excluded from the pixel matrix.
