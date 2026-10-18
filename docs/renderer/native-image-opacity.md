# Native image opacity investigation

Open UI never runs JavaScript. The consuming application constructs these
states with public Rust methods. Chromium 147 supplies the separate pixel and
geometry reference.

The [opacity consumer](../../bindings/rust/openui/examples/native_image_opacity.rs)
uses `Document`, typed element setters, `bounding_rect`, `render_to_png_buffer`,
`Element::on`, and `Element::click`. Its callback renders an initial state,
changes the image's opacity through Rust, and checks that the callback ran once.
It captures a weak element handle and checks document release at application
teardown, so the stored callback does not keep its own document alive.
The [paint overflow consumer](../../bindings/rust/openui/examples/native_image_opacity_ink.rs)
adds native shadows, outlines, transforms and overflow clipping. Both consumers
use the same retained document as other native applications.

## Measurements

Each state is compared at device scales 1, 1.25, 1.5, 2 and 3. The new
Chromium captures have two identical screenshots and two identical geometry
queries per state. Their inputs contain no scripts. The
[evidence index](generated/native-image-opacity-v1.json) records hashes,
source identities, differences and rejected experiments.

| Suite | Current renderer `9b158cda` | Unbounded prototype | Prototype with expanded paint bounds |
|---|---:|---:|---:|
| Existing opacity and transparent-background states | 40/160 exact | 160/160 | 160/160 |
| Overflow, nested groups, parent color, sibling and callback states | 0/160 exact | 160/160 | 160/160 |
| Shadows, outlines, transforms and parent clipping | 0/140 exact | 78/140 | 90/140 |
| Plain image guard | 120/120 exact | 120/120 | 120/120 |

All measured owned image rectangles match Chromium. The plain image guard
keeps its preceding pixels and geometry. These are development measurements;
the renderer prototypes are not applied to the framework.

## Shared causes isolated so far

The reduced opacity failures have three separate causes:

- Image opacity groups use an F16 intermediate where Chromium uses packed
  N32 color. Changing the shared content classification fixes the associated
  byte rounding.
- The fallback image's legacy inverse matrix must use its opacity group's
  raster origin. Floating-point translation followed by fixed-point stepping
  can select a different sample near a filter boundary. Chromium encloses
  drawable bounds in CSS coordinates before applying device scale; an empty,
  transparent parent does not enlarge those bounds.
- An opaque image does not completely hide its parent's background at an
  antialiased edge. Removing that parent color changes the final pixel.

The preserved prototypes change shared paint and image sampling operations.
They contain no test IDs, tolerances or replacements for captured references.
The checked-in diagnostic patches use zero context; reproduce them in an
isolated checkout with `git apply --unidiff-zero`.
Pinned Chromium `PaintChunker`, `PaintChunksToCcLayer`, and Skia
`SkBitmapProcState` sources support the drawable-bounds and sampling analysis.
The raw source downloads and their byte hashes remain in the evidence index.

## Remaining failures and rejected changes

The expanded-bounds prototype improves 139 of the 140 additional paint
comparisons against the current renderer. One worsens: the
[clipped image](evidence/native-image-opacity-ink-v1/clip-parent-opacity50-0/test.html)
at 1.25 scale changes from 83 wrong pixels to 105. Its right background edge
is one channel step too dark after clipping and opacity. The complete cause
and correction remain under review in the paint subsystem. Shadow, outline
and transform cases also retain differences.

Passing the estimated paint rectangle directly to `saveLayer` was rejected.
It worsens five transformed-child comparisons and changes a previously exact
2× comparison to 223 wrong pixels. Estimated box extents are not sufficient
to bound a layer containing transformed paint. The unbounded prototypes and
the rejected bounded variant remain preserved separately.

The first unbounded prototype retains exact focused and primitive matrices
at all 40 profiles: 640/640 and 960/960. The completed 1,920-case selection
is 7,039/7,680 exact, with 641 differences and zero render errors. Every
Open UI image, Chromium image and oracle identity matches the same subset
of the clean `9b158cda` census. Its matrix source differs from its build snapshot
only because the paint tests wrote a historical rendered PNG before matrix
launch.
The audit verifies that difference and preserves the test output; these dirty
reports do not satisfy clean-source qualification.

The clean original census remains 21,308/22,924 exact. This investigation
does not close the original pixel gate, residual ownership, the remaining
native API review, or release qualification. Needed element behavior must be
callable through public Rust methods and callbacks; a test-only Engine path
does not complete an application API.
