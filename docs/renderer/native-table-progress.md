# Native repeated-table body progress

Open UI executes no JavaScript. Consuming applications create and mutate
elements through public Rust methods and supply Rust event callbacks. Chromium
is the separate rendering and geometry reference.

## Measured behavior

The pinned Chromium oracle was queried twice for four viewports, with outer
column heights of 40, 30, 20 and 40.5 CSS pixels. Each run measured the table
before shrinking its body from 100 to 50 pixels, after that mutation, and after
restoring 100 pixels. All 96 observations agree between the independent runs.
No screenshots or native raster commands were executed by this query.

At the initial body height, Chromium produces 41, 51, 61 and 41 table fragments,
respectively. Repeated header and footer space does not consume the body's
source content. A continuation advances by at least one CSS pixel, even when
the geometric space left for the body is only half a pixel. The final body
slice may be smaller: the 40.5-pixel case ends with a half-pixel slice. Shrinking
the body produces one, one, eleven and one fragments; restoring it restores
the original geometry.

This follows Chromium's pinned `ClampedToValidFragmentainerCapacity` and
`ReserveSpaceInFragmentainer` implementations. The
[fragmentation rules](https://www.w3.org/TR/css-break-3/#breaking-rules) specify
a minimum capacity for progress; the
[table rules](https://www.w3.org/TR/css-tables-3/#repeated-headers) describe
repeating headers and footers. Exact qualification uses measured pinned
Chromium behavior.

## Prepared shared implementation

Clean private source `4b3cb72c` removes the enclosing declared-column-count cap
from the nested repeated-table flow. It computes a finite continuation count
from remaining body content, advances the source coordinate after every
continuation, and preserves the final table's body and footer extent. Normal
flow materialization also gives the body descendants their owned fragment
geometry. Positioned and monolithic descendants retain their separate policy.
The implementation changes shared layout code, with no test-ID selection or
post-raster pixel correction.

Test-only baseline `4318f606` includes an Engine regression and a consuming
Rust application. The application uses typed element setters, `client_rects`,
Rust click callbacks, ordered height mutations, detach/reattach and weak
ownership checks. An internal fixture is insufficient to complete a needed
public native API.

All eleven read-only generator, accountability, archive and formatting checks
pass. The [terminal review](generated/native-review-v3.json) records the failed
native guard: both baseline and corrected source produce four table fragments
where Chromium produces 41 at outer height 40 and body height 100. Both named
assertions exit 101 after successful compilation. The whole owner stops with
exit 1; workspace tests, native application and pixel matrices do not execute.
All seven own-source hosted hardening jobs pass with zero skips, which does
not resolve the native geometry failure.

Two further Chromium runs verify the application's absolute positioning with
96 geometry queries. Every query agrees between runs; the table and body
rectangles at the application's viewport also agree with the earlier margin
placement. The expected fragment count remains 41. Investigate the shared
continuation and fragment propagation before another qualification run; keep
the failed source, assertions and probes unchanged.

The [preserved evidence](generated/native-table-progress-v1.json) contains the
queries, unchanged inputs, source patches, checks and verification probes.
This candidate is unapplied. It establishes no new native execution pass,
pixel pass or release admission. The accepted renderer's full pixel gates
continue to fail.

## Canonical source retention follow-up

The [new evidence](generated/native-table-source-v1.json) uses a verified,
already built `e0dc491e` executable's geometry-only debug mode. Two independent
runs agree at all four profiles, with no raster commands or screenshots.
The first table has a 140-pixel source height and 20-pixel header/footer groups.
Its body descendant retains 100 pixels, while the visible direct row has been
cropped to 60 pixels. `repeated_table_sections` reads the cropped direct row's
height; the ancestor flow calculation then receives that partial extent.
The four projections are independently recomputed from the unchanged logs.

This diagnostic fixture produces five table fragments. The separate native
Engine guard above produces four at its own source. The diagnostic identifies
a source-data problem; it does not replace the failed guard's result or prove
the proposed correction. The first attempt used the matrix wrapper's backend
flag with the executable and stopped before layout. Its failed probe, receipt
and log remain preserved alongside the corrected debug run.

Clean private `e389b26a` rebases the failed progress trial onto umbrella
`1d846e68`. Test baseline `cd6d80be` therefore includes the earlier, unsuccessful
continuation change. The new correction retains one immutable unsliced table
subtree, shared by its fragments. Ancestor extent calculation uses its full
section geometry; each ancestor slice restores the canonical subtree before
applying its body window. The retained source never points back to its slices.
The Engine guard also requires all slices to share one source and checks that
teardown releases it. Existing expected counts and rectangles are unchanged.

Eleven read-only checks pass, including all generators and explicit formatting
for the changed Rust files. All 113 C exports and all 30 C layouts stay
unchanged. Whole owner `1601` waits for every stage of all 31 preceding owners.
It then requires the named baseline failure, fixed geometry and neighboring
fragmentation guards, a clean workspace, Rust/C/C++ consumers, sixty exact
Rust callback images against 240 stable Chromium captures, and all four pixel
matrices. Native geometry, ownership and pixels remain pending. The correction
is unapplied and admits no release state.

The same index preserves completed keyword-candidate hardening: seven passing
jobs, zero skips. Umbrella `1d846e68` completes three workflows with six passing
jobs and five skips. Captured hosted logs are stored with lossless gzip; each
decompressed SHA-256 is checked against the unchanged original capture. Those
hosted results do not qualify the new table source or its pending pixel work.
