# Native input and large-text glyph regressions

Open UI executes no JavaScript. These consuming applications create elements,
change typed styles, handle Rust click callbacks, and read owned geometry and
style snapshots through public Rust APIs. Needed browser-style element behavior
must be implemented in the shared Rust engine and exposed to the consuming app.
Pinned Chromium provides the expected pixels in a separate process.

## Later editor-clip investigation

The [native editor follow-up](native-input-editor-scrollport.md) separates the
anonymous input clip from the rejected glyph changes. On the unchanged umbrella
parent, it improves 22/30 native images to 28/30, with all thirty rectangles
exact. Its own complete renderer checks and remaining failures are recorded
there. The sources below remain historical, private and rejected.

## Earlier completed result

The [earlier complete glyph trial](native-glyph-mask.md) improved many original
comparisons but introduced three exact losses. This follow-up reduces the input
and large-text failures to ordinary consuming Rust apps. The corrected reduction
has thirty states across five profiles, fifteen callbacks per run, repeated
images, unchanged owned snapshots, and document teardown checks.

| Clean private source | Change | Exact images | Exact geometry |
|---|---|---:|---:|
| `4839e107` | Reduced input, ordinary large text, and sticky large text | 12/30 | 30/30 |
| `2a75fdc1` | Honor resolved monochrome hinting; retain required interpreters | 17/30 | 30/30 |
| `840b82ab` | Preserve fractional origins for unhinted outlines | 19/30 | 30/30 |
| `0c869236` | Retain actual strike size with normalized outlines | 19/30 | 30/30 |
| `e811f3d9` | Retain unhinted font-unit outlines; divide hinted coordinates directly | 20/30 | 30/30 |
| `4ae50fdd` | Retain the logical canvas transform for unhinted outlines | 20/30 | 30/30 |

The last two trials have identical native image bytes in all thirty states.
Their eight exact gains lose no previously exact reduced image, but both desktop
input states get worse: each changes from 68 to 136 differing pixels. Ten states
remain different. Every native pixel gate exits **1**. These sources are
**private, unapplied and rejected**; aggregate gains do not qualify them.

Each of the five new sources passes its own eighteen read-only checks. Their
complete original, expanded, focused and primitive matrices, workspace tests,
and C/ABI qualification are **not run**. Prior source results do not qualify
these sources. Accepted original results remain **21,334/22,924 exact**, with
1,590 differences and zero errors. No release state is admitted.

## Correct pixel direction

The input has an extra black row. Large text has an **extra white row** where
Chromium is green. The earlier description that sticky text loses a row was
incorrect and is corrected in the current documentation.

Every pixel in the two full-census regression regions was checked directly:

| Profile | Region | Native RGBA | Chromium RGBA |
|---|---|---|---|
| 1280×720 at 1.25 | x=25, y=384, width=1,440, height=1 | 255,255,255,255 | 0,128,0,255 |
| 1920×1080 at 1.5 | x=30, y=30, width=2,592, height=1 | 255,255,255,255 | 0,128,0,255 |

The reduced before states reproduce these bounds and colors in both an ordinary
large text block and its sticky version. The large-text failure therefore does
not require sticky positioning. Text and paint own this investigation, with
native controls owning the input's anonymous editing content. Select-control
regressions from the complete trial still require their own reduction.

## Baseline and strike evidence

Eight fresh Chromium captures measure the two relevant large-text profiles,
before and after the Rust callback's matching font change. An invisible inline
marker reads Chromium's baseline. Every PNG remains byte-identical to the
unchanged reference. The original before-state baselines are 250 logical pixels
at font size 288 and 366 at font size 432. The clean glyph source's fresh
fragment trace has the same baselines. Baseline placement alone does not explain
these two failures; paint origins, strike construction and raster coverage remain open.

The pinned
[Fontations scaler](https://skia.googlesource.com/skia/+/abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0/src/ports/SkTypeface_fontations.cpp)
honors unhinted monochrome fonts unless their contours require an interpreter.
The old adapter always used strong hinting. Another issue is its size-1 custom
font: Skia's
[mask or path decision](https://skia.googlesource.com/skia/+/abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0/src/core/SkStrikeSpec.cpp)
uses the declared font size and transform. The reviewed decision, glyph painter,
and custom-typeface files match Open UI's existing Skia pin byte-for-byte.
Correcting these representations improves the reduction but does not close its
pixel gate. No new font-name, font-size, or test-ID condition is introduced.

## Preserved failed attempts and limits

The first Rust reduction also exposed root percentage sizing with padding and
scrollbars: only ten of thirty rectangles matched and no full image was exact.
The second reduction uses a fixed viewport and hidden root overflow to isolate
text. That root sizing case remains unfinished native framework work.

The first baseline probe failed on a missing harness import. Its retry failed
the immutable-image check because a zero-width marker changed wrapping before
an overflowing word. Both failures are retained. The successful probe covers
only the two profiles where the word fits and all reference bytes stay fixed.
No failed probe is a pass.

The [evidence index](generated/native-glyph-regressions-v1.json) and
[completed archive](evidence/native-glyph-regressions-v1/completed-evidence.tar.gz)
retain the applications, lockfiles, patches, source snapshots, logs, repeated
native and Chromium images, per-state difference regions, channel deltas,
primary sources, and failed attempts. The archive has 2,005 members; every
member's bytes are independently verified. The
[pixel direction review](generated/native-glyph-direction-v1.json) records the
full-census color correction. Existing archives and Chromium images are
unchanged. All local owners are terminal and every measured source stayed clean
and unchanged throughout its pipeline.
