# Native font engine and physical outline investigation

Open UI never executes JavaScript. A consuming app creates and changes its
document through public Rust methods and Rust callbacks. Chromium is the
separate pixel reference, with zero tolerance. Historical Open UI images remain
immutable provenance.

## Completed LCD trial

Private `52788b83` now completes both full suites:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 20,771 / 22,924 | 2,153 | 0 | 1 |
| Expanded | 21,570 / 23,728 | 2,158 | 0 | 1 |

Against its intrinsic-sizing parent, each suite loses 493 exact comparisons
and gains none. There are 630 changed comparisons across 160 original test
IDs. All Chromium pixels, oracle identities, fonts and resources stay fixed.
All 22,924 original rows agree between the two suites. All 804 additions remain
unchanged from that parent, with 199/201 cases exact at all four profiles.
The focused gate fails at 600/640; primitive remains 960/960. The native static
app fixes 24/60 LCD images, with every owned bound and repeat unchanged.

The [versioned evidence](generated/native-scroll-insets-v30.json) records the
terminal pipeline and both audits. The complete audits are compressed without
altering their decompressed bytes:
[original](evidence/native-font-engines-v1/lcd-full-audit-v1.json.gz),
[expanded](evidence/native-font-engines-v1/lcd-expanded-audit-v1.json.gz).
This source is unapplied and unqualified.

## Shared font selection discrepancy

The [capture harness](../../tools/accountability/run_all_pixel_comparisons.py)
explicitly selects `FontDataServiceLinux:typeface/Freetype` for real-font
fixtures. Every one of the 493 newly lost exact comparisons uses that profile
and `cpu-skia:chromium-linux-lcd`.

The trial changed authored LCD text to custom Fontations outlines. That selected
a different typeface engine from the captured reference. The earlier native
static oracle uses a registered Ahem webfont and does not select the FreeType
feature. Its results therefore cannot justify switching every authored run
to Fontations.

There is also a physical-size discrepancy: LCD outlines are fitted at logical
font size and then stretched during scaled replay. Chromium's Fontations
source builds its hinting instance at physical strike size. The
[source review](evidence/native-font-engines-v1/font-engine-source-review-v1.json)
preserves the excerpts and hashes. It distinguishes the local Chromium source
version from the immutable capture executable. These observations identify
shared corrections; pixel causation still requires the native and full-suite
verification below. Text and paint own that verification.

## Prepared native correction

Clean private `0601cd30` preserves the existing FreeType choice, adds an
explicit immutable Fontations choice, and fits compatible outlines once at
physical strike size. The [reviewable patch](evidence/native-font-engines-v1/native-font-engines-v1.patch)
contains no fixture-ID selection or output pixel correction.

The [consuming Rust app](evidence/native-font-engines-v1/native_font_raster.rs)
selects its policy before creating a document. It creates 64 text positions,
renders them, changes text and color through one Rust click callback, reads
owned bounds, and checks that weak handles expire after teardown. The prepared
sweep covers four font families, five sizes and five scales under both explicit
engines: 400 complete images and 25,600 glyph states. The 64 phases are logical
1/64px positions; no claim of every possible physical strike phase is made.

Fresh diagnostic Chromium references use the same explicit fontconfig and
font bytes for both engines. They do not replace the original oracle inputs.
Each image is captured twice in each of two independent Chromium processes;
the native app also runs twice. Existing static references remain unchanged.

Ten read-only repository checks pass. The baseline physical-outline guard,
fixed guard, compilation and native pixel verification have not run yet.
The pipeline waits for the entire cache and fieldset pipelines to finish;
local Cargo builds and image sweeps remain separate. It then preserves the
1,000 native sizing measurements, checks 880 selected original comparisons,
both complete 40-profile matrices and both complete censuses.

The [versioned C raster-configuration transport](../v02/native-c-raster-configuration.md)
is prepared at clean private `3395cefa`, with a reviewable patch and owned
configuration over the same Rust engine. All 113 preceding exports and 30
layouts remain intact in the generated metadata; the candidate adds four
exports. Its five boundary tests and native C/C++ consumers are uncompiled.
The planned shared Rust/C/C++ sweep covers 1,200 images and 76,800 logical phase
states against unchanged Chromium references. Runtime, Miri, pixel qualification,
every field's rendering behavior and default native pixels remain required.

This private source also inherits the other intrinsic
regressions and the cache candidate. It is unapplied, admits no release case,
and does not change the accepted renderer's totals or release status.
