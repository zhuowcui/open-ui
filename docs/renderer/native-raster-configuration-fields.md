# Native text raster configuration

Applications configure and change their retained documents through public Rust
methods and Rust callbacks. Open UI executes no JavaScript. Chromium remains
the sole pixel target, with zero tolerance.

## Prepared shared renderer correction

Clean private `bcb7063f` follows the C configuration consumer correction
`ac08ec56`. Its [versioned evidence](generated/native-scroll-insets-v33.json)
preserves the [source patch](evidence/native-raster-fields-v1/native-raster-fields-v1.patch),
test-only baseline, native app, verification programs and waiting pipeline.
It is unapplied. Compilation, baseline/fixed regression tests, native execution
and fresh pixel comparisons remain pending.

The source review found that author and embedded phase fields were ignored,
native phase was largely cancelled by origin subtraction, and custom
Fontations outlines overrode requested text settings. This candidate changes
the shared renderer:

| Setting | Prepared behavior |
|---|---|
| LCD phase | Offset glyph ink horizontally by the requested 1/64 physical pixel units after fitting layout-origin positions. Logical advances, boxes and hit testing retain their layout coordinates. |
| Edging and positioning | Carry the configured mask, subpixel positioning and linear-metric flags into the custom Fontations font. |
| Hinting and autohint | Choose unhinted, light, normal, full or monochrome fitting from the configured font; select the interpreter or automatic hinter using Skia/Fontations rules. |
| LCD orientation | Carry surface pixel geometry to full LCD outline fitting. |
| Explicit Fontations backend | Preserve this selection for aliased and grayscale author text as well as LCD text. |

Phase is shared by horizontal, vertical, combined and emphasis painting. It
applies to subpixel-antialiased ink; aliased and grayscale masks ignore the
LCD-specific offset. Two private 10px-only phase/origin adjustments and their
calibration tests are removed. Physical-strike fitting and configured origins
must establish correct pixels through the unchanged Chromium gates.

The fitting logic follows the available local Skia Fontations source. That
checkout and the pinned executable are different patch versions; source review
does not establish agreement with the executable. All pixel comparisons still
use pinned Chromium `147.0.7727.50` and independently recorded inputs.

## Verification

The test-only baseline is `ee3d4134`. Its two new tests must fail by name, rather
than merely fail to compile. The correction must pass both tests and the
existing physical-outline neighbor. They check all three text roles, five
scales, integer physical phase shifts and requested mask/position flags.

The [native Rust application](evidence/native-raster-fields-v1/native_raster_fields.rs)
uses public immutable options, element setters and a Rust click callback. It
keeps owned geometry snapshots and verifies document teardown. Its planned
840-case application run checks repeatability, mask behavior, physical phase
distance and unchanged layout geometry. These API checks admit no Chromium
pixel cases.

Separate fresh Chromium comparisons cover 200 font cases, 400 images and
25,600 logical origin states, followed by the complete original, expanded,
focused and primitive matrices. Existing intrinsic and static native consumers
are retained. The new pipeline waits until every command in the earlier
clean-cache, fieldset, font and C configuration pipelines is terminal.

All ten read-only repository checks pass. The first check run found generated
author-style inventory drift after emphasis began using a shared settings
helper. Normal regeneration corrected it; both check receipts are preserved.
Rust formatting passes. No runtime or pixel pass is inferred from those checks.

## Open work

The candidate still needs compilation and consuming-app verification. Existing
family/size hinting and FreeType aliased-profile overrides require review.
Default Skia native text still bypasses the explicit CPU physical-strike branch
for custom outlines. Variable, synthetic, color-font and transformed/vertical
text behavior also need qualification. Copying configuration into the Engine
does not prove those behaviors complete.

The accepted umbrella renderer remains 21,334/22,924 exact, with 1,590
differences and zero errors. This candidate changes no accepted result, export
count or admitted release case. Every required native API and pixel gate
remains required.
