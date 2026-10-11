# Native inline image sizes

Applications load images, set typed styles and handle events through public
Rust methods and Rust callbacks. Open UI executes no JavaScript. Chromium is
the sole pixel reference. A missing native operation or incorrect native
result remains framework work even when a Chromium test uses a script.

## Observed failure

The image candidate `1413a862` stops at its fixed Engine geometry assertion.
An image with `Display::Inline`, a decoded 200 × 200 resource and explicit
150 × 150 dimensions reports `(20, 5, 0, 19)`. Its parent is
`(20, 20, 150, 150)`. The block iteration passes the preceding assertion.
The original geometry and pixel assertions are preserved. The candidate
has no completed native application or pixel matrix result.

The shared inline collector treats replaced content as an ordinary inline
container and walks its children. An image contributes its own box, including
its authored and intrinsic dimensions. The existing atomic inline path
already measures that box. The
[CSS sizing rules](https://www.w3.org/TR/CSS2/visudet.html#inline-replaced-height)
describe sizing for inline replaced content.

## Isolated correction

Private `c92e2d08`, based on the current umbrella checkpoint, routes inline
replaced content to the shared atomic inline path. It uses retained resource
metadata and the existing replaced-element classification. Display-none and
out-of-flow handling run first. Ordinary inline containers continue to collect
their children. No intrinsic-constraint or whitespace changes from rejected
`a6d386e4` are included.

The [versioned review](generated/native-review-v2.json) preserves the source
patch, test-only baseline, consuming applications and qualification probes:

- The named baseline test fails with actual exit 101 at the expected box
  assertion. Both corrected geometry and ordinary-inline neighbor tests pass.
- All thirteen read-only checks pass, including generators, accountability,
  Rust formatting and C/C++ syntax.
- Two independent pinned Chromium processes produce the same 48 geometry
  observations for image, canvas and SVG before and after a width change,
  across all four required profiles.
- The Rust application changes image width from a Rust click callback, keeps
  owned bounds, hides and restores the image, detaches and reattaches it, and
  verifies document teardown. C/C++ consumers call the same Engine through
  the existing ABI and retain callback user data until listener destruction.

The [completed native application evidence](generated/native-inline-replaced-v1.json)
records a clean build after clearing all eighteen workspace packages: 8,538
tests pass, zero fail and thirteen are ignored. All twelve C and six C++
consumers pass, preserving 113 exports and the existing layouts. The Rust
callback application has sixty of sixty images and bounds exact at five
scales and three subpixel origins. Two native runs and 240 Chromium captures
are stable. All callback, owned-bound, hide/restore, detach/reattach and
teardown assertions pass. The 503 preserved artifacts include every measured
native image and repeated Chromium capture.

## Completed matrix review: rejected

The [terminal review](generated/native-review-v3.json) records all seven stages.
Focused 640/640 and primitive 960/960 comparisons are exact. The original
census is unchanged at 21,334/22,924 exact, 1,590 different and zero errors.
The expanded census is 22,133/23,728 exact, 1,595 different and zero errors:
four previously exact comparisons fail, with no newly exact comparison. Both
full gates exit 1. Every Chromium image and oracle identity remains unchanged.

All four losses belong to `multicol-on-broken-image-alt-text`. Its retained
image has fallback children and no decoded replaced resource. The generic
image-tag classification sends that fallback host through atomic inline
layout, exposing its authored red background outside the green cover. The
shared layout must distinguish an actual replaced image from fallback flow.
The reduced resource-backed app does not exercise this fallback behavior.

The candidate is rejected for application to the umbrella branch. Its source,
tests, captures and failed results remain preserved. Accepted renderer counts
and release admission are unchanged; the full exact Chromium gate still fails.

The [fresh fallback follow-up](native-inline-fallback.md) is prepared on the
current umbrella source with native Rust/C resource-clearing APIs. Its
Chromium geometry and read-only checks pass; native execution and full pixel
qualification remain pending. The rejected source and its failed results are
preserved unchanged.
