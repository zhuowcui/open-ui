# Native glyph coverage and physical strikes

Open UI applications use public native Rust methods and Rust callbacks.
Open UI never executes JavaScript. Pinned Chromium defines the expected
pixels, with zero tolerance.

## Measured difference

The [preserved audit](generated/native-glyph-descriptor-v1.json) examines
200 existing Rust images and their unchanged Chromium references from the
clean `41b616c3` native application matrix. It creates no new screenshots.
All actual C/C++ images match Rust, and all repeated captures agree.

The native images all use equal red and green coverage at glyph edges.
Chromium uses different red and green coverage in 184 images. Both authored
colors have equal red and green components, so this is evidence of a shared
text coverage difference. All images are opaque. The other 16 Chromium images
contain visible grayscale glyphs at 20px and 24px with a scale of 3.

Open UI's measured default is Skia with grayscale antialiasing, slight
hinting, unknown pixel geometry, gamma 1 and contrast 0. This default does
not reproduce the captured Chromium glyph coverage. Matching Rust and C
output does not establish matching Chromium output.

## General strike correction under review

The pinned Skia source makes its LCD mask decision using the physical font
size and transform. Its ordinary LCD size limit explains why selecting LCD
for every image would also be incorrect. The native Fontations adapter
currently fits paths at their physical size, then replaces the font's size
with 1. That loses information required by Skia's mask decision.

Private candidate `fce42e08` keeps the original font size in the strike
descriptor and normalizes the fitted outlines and metrics before replay.
It carries the configured edging, hinting and pixel geometry through outline
creation, and applies the configured phase once at paint time. It also removes
the old 10px phase and origin overrides. The production change adds no font,
size, fixture or test-ID condition. Raster defaults and public Rust/C APIs
remain unchanged.

An identical guard on baseline `54bcdeab` and the candidate checks physical
strike descriptors across four real font families, five sizes and five
scales. The [completed native guard run](generated/native-glyph-descriptor-v5.json)
fails on the baseline with exit 101 and passes on the candidate. All 343 text
tests pass, zero failed and zero ignored. It cleans all 18 workspace packages
before each source and holds the exclusive owner across all five stages and
gaps. That guard-only record did not execute the full workspace, consuming
apps or pixel matrices. The candidate is unapplied and unqualified.
Its [fourteen read-only source checks](generated/native-glyph-descriptor-v3.json)
pass, including generated contracts, accountability, formatting and C/C++
syntax. These checks do not compile or qualify the Rust glyph implementation.

The first mutable draft placed this guard outside the test module; its
preflight failed before any build or render. The failed bytes and receipt
are preserved. The corrected source is separately committed and identified.

A later source review finds that `2f53d5de` copied a test-only configuration
constructor absent from the current public API. Its text tests cannot compile.
Fresh `fce42e08` uses the existing `chromium_linux_lcd` constructor; production
code is unchanged. Both sources and the finding remain preserved. Passing
read-only checks or hardening jobs that do not compile these text tests does
not qualify the guard.
The [complete `2f53d5de` hardening record](generated/native-glyph-descriptor-v4.json)
has seven jobs passed and zero skipped, with all job logs preserved. It does
not execute the text crate's unit tests and does not qualify `fce42e08`.

## Fresh trial on the integrated source

The [complete trial evidence](generated/native-glyph-descriptor-v6.json)
records fresh baseline `db03c8fa` and candidate `c68d946c` from umbrella
`16187f4f`. They contain the same consuming Rust app and descriptor guard.
The candidate ports the general correction without the previously rejected
width and raster changes. All fifteen read-only checks pass on each source.
The baseline reproduces the descriptor failure with exit 101. The candidate
passes that guard, all 343 text tests, and 8,560 workspace tests with zero
failures and 13 ignored. All eight candidate build stages pass. ABI verification
runs thirteen C and seven C++ consumers, retaining 113 exports and 30 layouts.

The Rust app creates 64 text positions, renders `X` in black, then uses a Rust
click callback to replace it with `XX` in blue. It checks immutable options,
owned bounds, callback count and document teardown. Each source runs the app
twice for four font families, five sizes, five scales and both default and
explicit LCD settings. No JavaScript executes in Open UI.

Both complete native matrices remain **0/400 images exact** against the
unchanged Chromium references. Each matches 23,040/25,600 geometry states;
126/25,600 position and state cells have exact pixels. Repeated native output
is byte-identical. The candidate's 200 default images remain unchanged from
the baseline and the earlier accepted native app. Under explicit LCD settings,
28 images change, all at a logical size of 10px: five have fewer differing
pixels and 21 have more. None becomes exact. Total differing pixels for that
group increase from 2,626,619 to 2,723,524. These changes do not qualify the
default text path or the candidate.

All eight pipeline stages are terminal. Focused 640/640 and primitive 960/960
remain exact, preserving all nine comparison invariants. The complete original
census finishes at 21,305/22,924 exact, with 1,619 differences and zero errors.
Expanded finishes at 22,108/23,728 exact, with 1,620 differences and zero errors.
Each complete suite loses 29 exact comparisons and gains none against the
accepted source. Both pixel gates exit 1. There are 63 changed comparisons
across 39 original test IDs; their reviewed causes and ownership remain open.
All Chromium PNGs, decoded pixels and oracle identities stay unchanged.
All original rows agree with expanded, and all 804 additions retain their
comparison invariants, with 200/201 cases exact at all four profiles.

The descriptor correction is rejected and remains unapplied. Accepted totals
remain 21,334/22,924 original and 22,137/23,728 expanded. The archive preserves
all first native PNGs, both geometry runs, actual process logs, original inputs
and unchanged first reference PNGs. Every duplicate native and reference PNG
is verified identical before its extra copy is omitted. Bounds, connected
regions, channel deltas and scale behavior remain in the native receipts and
audits. Passing API assertions does not establish Chromium pixel equality.

## Typeface selection still needs implementation

The saved native references explicitly select Chromium's Fontations typeface
engine. The general real-font renderer references explicitly select FreeType.
Both are immutable Chromium inputs. The current `chromium_linux_lcd`
constructor keeps authored LCD text on FreeType, matching the latter capture
conditions. It provides no explicit Fontations constructor for the native app.

The reviewed source confirms that authored LCD text can bypass the custom
Fontations adapter. That observation alone does not prove an incorrect
FreeType choice for the general renderer references or explain every pixel
difference. An explicit native Rust Fontations choice, correct default native
text, and the shared strike implementation still need implementation and
qualification against their unchanged reference conditions. Changing the
oracle's typeface choice would not close this gap. No runtime dispatch trace
or new formal WPT residual owner is claimed.

The [private native choice draft](generated/native-font-choice-v1.json) at
`ef8880b0` adds `RasterConfiguration::chromium_linux_fontations_lcd` over the
same Engine. It retains the immutable choice with resolved fonts and routes
it through horizontal, vertical, shadow and combined text painting. It adds
no font-family, size, fixture or test-ID rule. Existing defaults and the
explicit FreeType choice stay unchanged. All fifteen read-only checks pass.
The draft changes outline rendering. Font matching, shaping and metrics
still use the existing Skia typefaces and need separate exact qualification.
The draft is uncompiled, unapplied and pixel-unqualified; it inherits the
rejected descriptor candidate. Consuming apps, all raster settings, color and
variable fonts, neighboring decoration and transform behavior, default native
pixels, all four matrices and hosted jobs remain required.

## Remaining work

This audit establishes the coverage discrepancy. It does not explain every
outline, origin, hinting or color difference, and assigns no new formal WPT
residual ownership. The default coverage policy, public native application
path, neighboring cases and complete renderer matrices still need exact
Chromium qualification. No release state is admitted.
