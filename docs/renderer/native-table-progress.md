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
pass. The baseline assertion, fixed guards, locked workspace, consuming
application and all four complete renderer matrices remain **pending**. Whole
owner `1548` waits for every prior whole Cargo/image pipeline to finish and
then executes those stages in order. The current source and its probes are
frozen for that run.

The [preserved evidence](generated/native-table-progress-v1.json) contains the
queries, unchanged inputs, source patches, checks and verification probes.
This candidate is unapplied. It establishes no new native execution pass,
pixel pass or release admission. The accepted renderer's full pixel gates
continue to fail.
