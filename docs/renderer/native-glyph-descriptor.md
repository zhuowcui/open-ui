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
gaps. The full workspace, consuming apps and pixel matrices remain unexecuted.
The candidate is unapplied and unqualified.
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

## Remaining work

This audit establishes the coverage discrepancy. It does not explain every
outline, origin, hinting or color difference, and assigns no new formal WPT
residual ownership. The default coverage policy, public native application
path, neighboring cases and complete renderer matrices still need exact
Chromium qualification. No release state is admitted.
