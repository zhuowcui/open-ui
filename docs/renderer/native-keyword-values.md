# Native fragmentation and border keyword values

Open UI never runs JavaScript. Applications set element state through public
Rust methods and Rust callbacks over the shared Engine. The C ABI forwards
native calls to that same engine. Every needed operation requires a working
public application API, including its state changes and events.

## Measured API gap

The corrected, nonrasterizing C diagnostic loads the verified existing library
from clean `e0dc491e`, checks every ABI layout it uses, and follows the public
headers. `oui_style_value_parse` rejects `ColumnFill` with the literal `auto`,
returning -1. The diagnostic exits 1 before any table geometry query completes.
Its preceding layout-preflight and header-adapter failures remain preserved.
None of those runs qualifies table geometry or pixels.

Rust applications already have the typed operation:
`element.set_column_fill(ColumnFill::Auto)`. The missing C value construction
is native framework work. Chromium test scripts do not waive that work.

## Prepared correction

Clean private `06e1f89a` rebases the earlier constructor candidate onto umbrella
`92741843`, following test-only baseline `02748036`. The style generator maps
every declared variant of seven shared enum types: border style, box decoration
break, break value, break inside, column fill, column span and column wrap.
Its generated native parser covers thirteen author-facing properties. The
display parser also accepts `inline-table` and the eight internal table roles.
The C boundary retains the compound value's property identity and ownership.

Generated property code comes from the shared style schema. No fixture IDs,
captured references, layout or paint implementations are changed. All 113
existing exports and all 30 ABI layouts remain unchanged. The correction is
unapplied; thirteen read-only generator, accountability, C/C++ syntax and Rust
format checks pass.

## Required verification

Exclusive whole owner `1588` waits for every stage of all thirty preceding
pipelines, including raster owner `1560` and fallback owner `1576`. It then
requires the named baseline assertion to fail, all three fixed guards to pass,
and a clean locked workspace plus public Rust, C and C++ consuming applications.

The Rust application changes typed styles in a Rust click callback and verifies
owned computed styles, bounds and teardown. Its ten before/after images at
five scales require two identical native runs, twenty independent Chromium
capture processes and forty stable captures, with zero pixel tolerance.
A separate C table geometry diagnostic uses the newly built verified library;
deterministic geometry alone does not establish equality with Chromium.
Focused, primitive, original and expanded pixel matrices all remain required.
That earlier owner is now terminal with exit 241. Its named baseline fails
with exit 101 as expected. All three fixed guards stop at the disk guard with
exit -15; none is a pass. Its native applications and pixels do not execute.
The [terminal evidence](generated/native-keywords-v2.json) preserves those
receipts and every guard log.

The [subsequent preserved checkpoint](generated/native-table-source-v1.json)
records all seven own-source hardening jobs passing at `06e1f89a`, with zero
skips. It preserves each job conclusion and the captured log bytes. Those
hosted passes do not replace the stopped local fixed guards; no native
application or pixel pass is claimed.

## Fresh current-API source

Clean private `7d6ffabf` carries the same constructor correction onto
`0733955a`, retaining the current shared Rust/C text setter, style inheritance
and renderer. Generated property code is regenerated from its generator.
Sixteen read-only checks pass, including both keyword consumers' C/C++ syntax
and formatting of all 60 tracked C/C++ files. The keyword consumers are
formatted with CI's clang-format 18.1.3; their earlier formatting failure is
preserved. CMake also includes the existing native text consumers.

The fresh source preserves all 113 exports and 30 ABI layouts.
Its [complete own-source hardening record](generated/native-keywords-v3.json)
passes all seven jobs, zero skips. Both named FFI keyword guards execute
successfully in the hosted platform job. All job logs remain preserved. These
hosted checks do not replace the source-identified local guards, consuming
applications and pixel matrices.

The [local guard and storage record](generated/native-keywords-v4.json)
reproduces the named baseline failure with exit 101 and passes all three
fixed guards. The first current-source build passes 8,554 workspace tests,
zero failed and 13 ignored, then the ABI checker reaches the storage guard.
Its temporary library copy and the older workspace Cargo cache are moved to
the data drive with all file bytes and original cache paths preserved. The
fresh build stores temporary compiler files on the data drive.

That [complete clean build](generated/native-keywords-v5.json) passes all
thirteen stages, 8,554 workspace tests, zero failed and 13 ignored. It runs
the consuming Rust, C and C++ keyword apps at all five scales. The ABI checker
also runs thirteen C examples and seven C++ examples. A source-path error in
the image probe stops the following stage before rendering. That failed
probe remains frozen; fresh probes verify each worker's source-root assignment
against the actual repository before launch. Native Chromium capture and the
focused, primitive, original and expanded pixel gates are still required.
The [complete follow-up](generated/native-keywords-v6.json) finishes all four
matrices at the tested `7d6ffabf` source: 640/640 focused, 960/960 primitive,
21,334/22,924 original and 22,137/23,728 expanded exact, zero errors.
All 48,252 rows preserve their nine comparison invariants, including every
Chromium input; no exact comparison is lost or gained. Both full gates
still exit 1, and the same 200/201 additions meet all four profiles.

The first native capture stops before the Chromium endpoint is available;
the first C diagnostic rejects a fractional viewport. Both failures remain
preserved. A fresh native capture uses a Linux filesystem for Chromium
profiles and temporary sockets, and the C diagnostic uses the native positive
half-away-from-zero viewport rounding. All ten native Rust images and bounds
match Chromium exactly at five scales, across two native runs and forty
stable captures from twenty independent Chromium processes. The C table
diagnostic completes 120 deterministic observations across two runs; it does
not establish equality with Chromium.

The verified constructor code is now integrated into umbrella `2d338d6c`,
with the public Rust raster options. Its [clean combined verification](generated/native-rust-options-v2.json)
passes fifteen build stages, 8,558 workspace tests and fifteen read-only
checks. Rust/C/C++ consumers run successfully, retaining 113 exports and 30
layouts. Both native Rust apps pass ten exact images and bounds each at five
scales. The earlier complete matrices retain their actual source;
combined-source complete matrices, full pixel parity and reviewed residual
ownership remain open. No release state is admitted.

The [preserved evidence](generated/native-keywords-v1.json) includes source
patches, immutable probes, failed C diagnostics and completed read-only checks.
It also records umbrella `92741843` CI: three successful workflows, six passing
jobs and five skips. The fallback candidate's seven own-source hardening jobs
pass with zero skips. Neither hosted result qualifies pending native or pixel
work. The accepted renderer totals and release admission remain unchanged.
